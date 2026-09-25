//! Raw Win32 calls: window enumeration, the `PrintWindow` fallback and COM
//! apartment setup.
//!
//! This is the only module allowed to use `unsafe`; every block carries a
//! `// SAFETY:` note. Callers see only `win32ui` types, never `windows` ones.

#![allow(unsafe_code)]

use core::ffi::c_void;
use core::mem::size_of;

use win32ui::{Hwnd, RgbaImage};
use windows::Win32::Foundation::{HWND, LPARAM, RECT};
use windows::Win32::Graphics::Gdi::{
    BI_RGB, BITMAPINFO, BITMAPINFOHEADER, CreateCompatibleBitmap, CreateCompatibleDC,
    DIB_RGB_COLORS, DeleteDC, DeleteObject, GetDIBits, GetWindowDC, HDC, HGDIOBJ, ReleaseDC,
    SelectObject,
};
use windows::Win32::Storage::Xps::{PRINT_WINDOW_FLAGS, PrintWindow};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetWindowRect, GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId,
    IsWindowVisible,
};
use windows::core::BOOL;

/// `PW_RENDERFULLCONTENT`, from `WinUser.h`. The `windows` crate exports it as a
/// bare `u32` in the messaging namespace, not as a `PRINT_WINDOW_FLAGS`, so it
/// is spelled here to reach the matching type.
const PW_RENDERFULLCONTENT: u32 = 2;

/// An STA COM apartment guard for `Windows.Graphics.Capture`.
///
/// win32ui does not initialise COM, but `capture_hwnd` needs an apartment on the
/// calling thread. Initialisation is best-effort: if COM is already up under a
/// different model the guard is inert, the composited capture fails and the
/// caller falls back to `PrintWindow`, which needs no apartment.
pub mod com {
    use windows::Win32::Foundation::{RPC_E_CHANGED_MODE, S_OK};
    use windows::Win32::System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx, CoUninitialize};

    /// Holds an STA apartment for as long as it lives, releasing it only when
    /// this call was the one that initialised COM.
    pub struct Apartment {
        owned: bool,
    }

    impl Apartment {
        /// Raises an STA apartment if the thread has none.
        pub fn initialize() -> Apartment {
            // SAFETY: the reserved pointer is null and the threading model is a
            // valid `COINIT` value; the matching `CoUninitialize` runs in `Drop`
            // exactly when this call returned `S_OK`.
            let result = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
            if result == RPC_E_CHANGED_MODE {
                Apartment { owned: false }
            } else {
                Apartment {
                    owned: result == S_OK,
                }
            }
        }
    }

    impl Drop for Apartment {
        fn drop(&mut self) {
            if self.owned {
                // SAFETY: paired with the `CoInitializeEx` that returned `S_OK`
                // and runs on the same thread.
                unsafe { CoUninitialize() };
            }
        }
    }
}

/// A predicate over `(process id, title)` used to pick a window.
struct Search<'a, F> {
    matches: Vec<usize>,
    predicate: &'a mut F,
}

/// Returns the first visible top-level window whose process id and title satisfy
/// `predicate`, in Z-order.
pub fn find_window<F>(mut predicate: F) -> Option<Hwnd>
where
    F: FnMut(u32, &str) -> bool,
{
    let mut search = Search {
        matches: Vec::new(),
        predicate: &mut predicate,
    };
    // SAFETY: `search` outlives the synchronous enumeration and the callback
    // only reaches it through the pointer passed here.
    let enumerated = unsafe {
        EnumWindows(
            Some(enum_proc::<F>),
            LPARAM((&mut search as *mut Search<'_, F>).cast::<c_void>() as isize),
        )
    };
    enumerated.ok()?;
    search.matches.first().map(|hwnd| Hwnd::from_raw(*hwnd))
}

unsafe extern "system" fn enum_proc<F>(hwnd: HWND, lparam: LPARAM) -> BOOL
where
    F: FnMut(u32, &str) -> bool,
{
    // SAFETY: `find_window` passes a pointer to a live `Search` that outlives
    // this synchronous enumeration; `EnumWindows` never calls it off-thread.
    let search = unsafe { &mut *(lparam.0 as *mut Search<'_, F>) };
    if !unsafe { IsWindowVisible(hwnd) }.as_bool() {
        return BOOL(1);
    }
    let mut pid = 0u32;
    // SAFETY: `hwnd` is live and `pid` is a correctly typed out-value.
    unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
    if (search.predicate)(pid, &window_title(hwnd)) {
        search.matches.push(hwnd.0 as usize);
    }
    BOOL(1)
}

/// The window's title, or an empty string when it has none.
fn window_title(hwnd: HWND) -> String {
    // SAFETY: `hwnd` is live for the duration of the enumeration.
    let len = unsafe { GetWindowTextLengthW(hwnd) };
    if len <= 0 {
        return String::new();
    }
    let mut buf = vec![0u16; len as usize + 1];
    // SAFETY: `buf` has room for `len` code units plus the NUL terminator.
    let written = unsafe { GetWindowTextW(hwnd, &mut buf) };
    if written <= 0 {
        return String::new();
    }
    String::from_utf16_lossy(&buf[..written as usize])
}

/// Renders `hwnd` with `PrintWindow(PW_RENDERFULLCONTENT)`, a cheap fallback that
/// does not need COM or DWM.
///
/// The captured region includes the frame and the alpha channel is forced to
/// 255. Returns `None` when the window has no drawable rectangle or the capture
/// failed.
pub fn print_window(hwnd: Hwnd) -> Option<RgbaImage> {
    let raw = HWND(hwnd.raw() as *mut c_void);
    let mut rect = RECT::default();
    // SAFETY: `raw` is a live handle and `rect` is a valid out-pointer.
    if unsafe { GetWindowRect(raw, &mut rect) }.is_err() {
        return None;
    }
    let width = rect.right - rect.left;
    let height = rect.bottom - rect.top;
    if width <= 0 || height <= 0 {
        return None;
    }

    // SAFETY: `raw` is live; `GetWindowDC` returns a DC owned by this call until
    // `ReleaseDC` below.
    let window_dc = unsafe { GetWindowDC(Some(raw)) };
    if window_dc.0.is_null() {
        return None;
    }
    // SAFETY: `window_dc` is live; the memory DC is freed below.
    let memory_dc = unsafe { CreateCompatibleDC(Some(window_dc)) };
    if memory_dc.0.is_null() {
        release(window_dc, raw);
        return None;
    }
    // SAFETY: `window_dc` is live; the bitmap is freed below.
    let bitmap = unsafe { CreateCompatibleBitmap(window_dc, width, height) };
    if bitmap.0.is_null() {
        let _ = unsafe { DeleteDC(memory_dc) };
        release(window_dc, raw);
        return None;
    }

    // SAFETY: `memory_dc` and `bitmap` are live; the previous object is put back
    // below before either is freed.
    let previous = unsafe { SelectObject(memory_dc, HGDIOBJ(bitmap.0)) };
    // SAFETY: `raw` and `memory_dc` are live and the bitmap is selected into it.
    let printed =
        unsafe { PrintWindow(raw, memory_dc, PRINT_WINDOW_FLAGS(PW_RENDERFULLCONTENT)) }.as_bool();

    let pixels = if printed {
        read_pixels(memory_dc, bitmap, width, height)
    } else {
        None
    };

    // SAFETY: `previous` came from selecting `bitmap` into `memory_dc`; restore
    // it so both can be freed without a dangling selection.
    unsafe {
        SelectObject(memory_dc, previous);
        let _ = DeleteObject(HGDIOBJ(bitmap.0));
        let _ = DeleteDC(memory_dc);
    }
    release(window_dc, raw);

    pixels.map(|pixels| RgbaImage {
        width: width as u32,
        height: height as u32,
        pixels: rgba_from_bgra(pixels),
    })
}

/// Copies a 32-bit DIB section's BGRA pixels out of `bitmap`.
fn read_pixels(
    dc: HDC,
    bitmap: windows::Win32::Graphics::Gdi::HBITMAP,
    w: i32,
    h: i32,
) -> Option<Vec<u8>> {
    let mut info = BITMAPINFO::default();
    info.bmiHeader.biSize = size_of::<BITMAPINFOHEADER>() as u32;
    info.bmiHeader.biWidth = w;
    // Negative height asks for top-down rows, matching `RgbaImage`.
    info.bmiHeader.biHeight = -h;
    info.bmiHeader.biPlanes = 1;
    info.bmiHeader.biBitCount = 32;
    info.bmiHeader.biCompression = BI_RGB.0;

    let mut pixels = vec![0u8; w as usize * h as usize * 4];
    // SAFETY: `dc` and `bitmap` are live, `info` is fully initialised and
    // `pixels` holds exactly `w * h * 4` writable bytes.
    let lines = unsafe {
        GetDIBits(
            dc,
            bitmap,
            0,
            h as u32,
            Some(pixels.as_mut_ptr().cast::<c_void>()),
            &mut info,
            DIB_RGB_COLORS,
        )
    };
    (lines != 0).then_some(pixels)
}

/// Converts straight BGRA (as `GetDIBits` returns it) to straight RGBA, forcing
/// the alpha channel opaque.
fn rgba_from_bgra(mut pixels: Vec<u8>) -> Vec<u8> {
    for pixel in pixels.as_chunks_mut::<4>().0 {
        pixel.swap(0, 2);
        pixel[3] = 0xFF;
    }
    pixels
}

/// Releases a window DC with the window it was obtained for.
fn release(dc: HDC, hwnd: HWND) {
    // SAFETY: `dc` came from `GetWindowDC(Some(hwnd))`.
    unsafe { ReleaseDC(Some(hwnd), dc) };
}
