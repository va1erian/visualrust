//! The control `HWND`: creation, the typed `SCI_*` sends, and RAII teardown.
//!
//! `Control` is `!Send` and `!Sync` because it holds raw window handles: a
//! Win32 window can only be touched from the thread that created it, and the
//! compiler enforces that here.

use core::ffi::c_void;
use core::sync::atomic::{AtomicBool, Ordering};

use windows::Win32::Foundation::{HINSTANCE, HWND};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DestroyWindow, WINDOW_EX_STYLE, WS_CHILD, WS_OVERLAPPED,
};
use windows::core::{PCWSTR, w};

use vr_scintilla_sys::sys::send_message;

use crate::messages::{
    SCI_ANNOTATIONSETTEXT, SCI_ANNOTATIONSETVISIBLE, SCI_AUTOCCANCEL, SCI_AUTOCSHOW,
    SCI_BRACEMATCH, SCI_CALLTIPCANCEL, SCI_CALLTIPSHOW, SCI_GETLENGTH, SCI_GETTEXT, SCI_GOTOLINE,
    SCI_INDICATORCLEARRANGE, SCI_INDICATORFILLRANGE, SCI_INDICSETALPHA, SCI_INDICSETFORE,
    SCI_INDICSETSTYLE, SCI_MARGINSETTEXT, SCI_MARKERADD, SCI_MARKERDEFINE, SCI_MARKERDELETE,
    SCI_SETCODEPAGE, SCI_SETILEXER, SCI_SETINDICATORCURRENT, SCI_SETINDICATORVALUE,
    SCI_SETMARGINTYPEN, SCI_SETMARGINWIDTHN, SCI_SETSTYLING, SCI_SETTEXT, SCI_STARTSTYLING,
    SCI_STYLESETBACK, SCI_STYLESETBOLD, SCI_STYLESETFORE, SCI_STYLESETITALIC, SCI_STYLESETSIZE,
};
use vr_scintilla_sys::Scintilla_RegisterClasses;

/// Document encoding: `SC_CP_UTF8` from `Scintilla.h`.
const SC_CP_UTF8: usize = 65001;

/// `SCI_STARTSTYLING` mask: set the low five style bits.
const STYLE_MASK: isize = 0x1F;

/// `INVALID_POSITION`, returned by `SCI_BRACEMATCH` when there is no match.
const INVALID_POSITION: isize = -1;

static REGISTER: std::sync::Once = std::sync::Once::new();
static REGISTERED: AtomicBool = AtomicBool::new(false);

/// Registers Scintilla's window classes exactly once per process, recording
/// whether it worked. Resources are never released: `Scintilla_ReleaseResources`
/// is only valid once every control is gone, and a live `Control` cannot know
/// about its siblings.
fn ensure_registered() -> bool {
    REGISTER.call_once(|| {
        // SAFETY: `GetModuleHandleW` only reads the process module list; a null
        // name asks for the executable's own module handle.
        if let Ok(module) = unsafe { GetModuleHandleW(None) } {
            // SAFETY: `module` is a live instance handle; registration requires
            // it once before any control is created.
            let ok = unsafe { Scintilla_RegisterClasses(module.0) } != 0;
            REGISTERED.store(ok, Ordering::SeqCst);
        }
    });
    REGISTERED.load(Ordering::SeqCst)
}

/// A Scintilla control and the hidden host window that parents it.
pub(crate) struct Control {
    hwnd: *mut c_void,
    host: *mut c_void,
}

impl Control {
    /// Creates a hidden `STATIC` host and a `Scintilla` child, or `None` when
    /// the session cannot create windows (for example a non-interactive
    /// desktop).
    pub(crate) fn create() -> Option<Control> {
        if !ensure_registered() {
            return None;
        }
        // SAFETY: `GetModuleHandleW` only reads the process module list.
        let module = unsafe { GetModuleHandleW(None) }.ok()?;
        let instance = HINSTANCE(module.0);

        // SAFETY: `instance` is the live module handle just fetched; the class
        // name is a static wide string. The host is never shown.
        let host = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE(0),
                w!("STATIC"),
                w!("vr-scintilla host"),
                WS_OVERLAPPED,
                0,
                0,
                0,
                0,
                None,
                None,
                Some(instance),
                None,
            )
        };
        let host = match host {
            Ok(host) => host,
            Err(_) => return None,
        };

        // SAFETY: `host` is live and `instance` owns the `Scintilla` class
        // registered above. The control is created but never shown.
        let control = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE(0),
                w!("Scintilla"),
                PCWSTR::null(),
                WS_CHILD,
                0,
                0,
                0,
                0,
                Some(host),
                None,
                Some(instance),
                None,
            )
        };
        let hwnd = match control {
            Ok(hwnd) => hwnd,
            Err(_) => {
                // SAFETY: `host` is live and owned by this call.
                unsafe {
                    let _ = DestroyWindow(host);
                }
                return None;
            }
        };

        let control = Control {
            hwnd: hwnd.0,
            host: host.0,
        };
        // SAFETY: `hwnd` is a live control; the code page takes only a scalar.
        unsafe {
            send_message(control.hwnd, SCI_SETCODEPAGE, SC_CP_UTF8, 0);
        }
        Some(control)
    }

    /// Sends a scalar-parameter `SCI_*` message to the control.
    ///
    /// Callers must pass `wparam` / `lparam` values that match `message`'s
    /// contract and must not pass raw pointers here; the pointer-carrying
    /// messages below take slices so the pointer stays valid for the call.
    fn send(&self, message: u32, wparam: usize, lparam: isize) -> isize {
        // SAFETY: `self.hwnd` is a live Scintilla control for as long as `self`
        // exists; `message` and its scalar parameters are chosen by the typed
        // methods in this module.
        unsafe { send_message(self.hwnd, message, wparam, lparam) }
    }

    // Text.

    pub(crate) fn length(&self) -> usize {
        usize::try_from(self.send(SCI_GETLENGTH, 0, 0)).unwrap_or(0)
    }

    pub(crate) fn get_text(&self, buffer: &mut [u8]) -> isize {
        let length = buffer.len();
        // SAFETY: `buffer` is valid for `length` writes and outlives this
        // synchronous call; Scintilla writes at most `length` bytes, including
        // the NUL terminator.
        self.send(
            SCI_GETTEXT,
            length,
            buffer.as_mut_ptr().cast::<c_void>() as isize,
        )
    }

    pub(crate) fn set_text(&self, bytes: &[u8]) {
        // SAFETY: `bytes` is NUL-terminated (the `codec` helpers guarantee it)
        // and outlives this synchronous call.
        self.send(SCI_SETTEXT, 0, bytes.as_ptr().cast::<c_void>() as isize);
    }

    // Styles.

    pub(crate) fn style_fore(&self, style: i32, color: i32) {
        self.send(SCI_STYLESETFORE, style as usize, color as isize);
    }

    pub(crate) fn style_back(&self, style: i32, color: i32) {
        self.send(SCI_STYLESETBACK, style as usize, color as isize);
    }

    pub(crate) fn style_bold(&self, style: i32, bold: bool) {
        self.send(SCI_STYLESETBOLD, style as usize, bold as isize);
    }

    pub(crate) fn style_italic(&self, style: i32, italic: bool) {
        self.send(SCI_STYLESETITALIC, style as usize, italic as isize);
    }

    pub(crate) fn style_size(&self, style: i32, points: i32) {
        self.send(SCI_STYLESETSIZE, style as usize, points as isize);
    }

    // Margins and markers.

    pub(crate) fn set_margin_type(&self, margin: i32, kind: i32) {
        self.send(SCI_SETMARGINTYPEN, margin as usize, kind as isize);
    }

    pub(crate) fn set_margin_width(&self, margin: i32, width: i32) {
        self.send(SCI_SETMARGINWIDTHN, margin as usize, width as isize);
    }

    pub(crate) fn set_margin_text(&self, line: i32, bytes: &[u8]) {
        // SAFETY: `bytes` is NUL-terminated and outlives this call.
        self.send(
            SCI_MARGINSETTEXT,
            line as usize,
            bytes.as_ptr().cast::<c_void>() as isize,
        );
    }

    pub(crate) fn define_marker(&self, marker: i32, symbol: i32) {
        self.send(SCI_MARKERDEFINE, marker as usize, symbol as isize);
    }

    pub(crate) fn add_marker(&self, line: i32, marker: i32) -> isize {
        self.send(SCI_MARKERADD, line as usize, marker as isize)
    }

    pub(crate) fn delete_marker(&self, line: i32, marker: i32) {
        self.send(SCI_MARKERDELETE, line as usize, marker as isize);
    }

    // Indicators.

    pub(crate) fn indicator_set_style(&self, indicator: i32, style: i32) {
        self.send(SCI_INDICSETSTYLE, indicator as usize, style as isize);
    }

    pub(crate) fn indicator_set_fore(&self, indicator: i32, color: i32) {
        self.send(SCI_INDICSETFORE, indicator as usize, color as isize);
    }

    pub(crate) fn indicator_set_alpha(&self, indicator: i32, alpha: i32) {
        self.send(SCI_INDICSETALPHA, indicator as usize, alpha as isize);
    }

    pub(crate) fn set_indicator_current(&self, indicator: i32) {
        self.send(SCI_SETINDICATORCURRENT, indicator as usize, 0);
    }

    pub(crate) fn set_indicator_value(&self, value: i32) {
        self.send(SCI_SETINDICATORVALUE, value as usize, 0);
    }

    pub(crate) fn indicator_fill(&self, start: i32, length: i32) {
        self.send(SCI_INDICATORFILLRANGE, start as usize, length as isize);
    }

    pub(crate) fn indicator_clear(&self, start: i32, length: i32) {
        self.send(SCI_INDICATORCLEARRANGE, start as usize, length as isize);
    }

    // Annotations.

    pub(crate) fn annotation_set_text(&self, line: i32, bytes: &[u8]) {
        // SAFETY: `bytes` is NUL-terminated and outlives this call.
        self.send(
            SCI_ANNOTATIONSETTEXT,
            line as usize,
            bytes.as_ptr().cast::<c_void>() as isize,
        );
    }

    pub(crate) fn annotation_set_visible(&self, visible: i32) {
        self.send(SCI_ANNOTATIONSETVISIBLE, visible as usize, 0);
    }

    // Autocompletion and call tips.

    pub(crate) fn auto_show(&self, len_entered: i32, bytes: &[u8]) {
        // SAFETY: `bytes` is NUL-terminated and outlives this call.
        self.send(
            SCI_AUTOCSHOW,
            len_entered as usize,
            bytes.as_ptr().cast::<c_void>() as isize,
        );
    }

    pub(crate) fn auto_cancel(&self) {
        self.send(SCI_AUTOCCANCEL, 0, 0);
    }

    pub(crate) fn call_tip_show(&self, position: i32, bytes: &[u8]) {
        // SAFETY: `bytes` is NUL-terminated and outlives this call.
        self.send(
            SCI_CALLTIPSHOW,
            position as usize,
            bytes.as_ptr().cast::<c_void>() as isize,
        );
    }

    pub(crate) fn call_tip_cancel(&self) {
        self.send(SCI_CALLTIPCANCEL, 0, 0);
    }

    // Navigation and the container lexer.

    pub(crate) fn goto_line(&self, line: i32) {
        self.send(SCI_GOTOLINE, line as usize, 0);
    }

    pub(crate) fn brace_match(&self, position: i32) -> Option<usize> {
        let result = self.send(SCI_BRACEMATCH, position as usize, 0);
        if result == INVALID_POSITION {
            None
        } else {
            usize::try_from(result).ok()
        }
    }

    pub(crate) fn set_ilexer_container(&self) {
        // A null lexer pointer selects the container lexer; no dereference
        // happens on Scintilla's side.
        self.send(SCI_SETILEXER, 0, 0);
    }

    pub(crate) fn start_styling(&self, position: i32) {
        self.send(SCI_STARTSTYLING, position as usize, STYLE_MASK);
    }

    pub(crate) fn set_styling(&self, length: i32, style: i32) {
        self.send(SCI_SETSTYLING, length as usize, style as isize);
    }
}

impl Drop for Control {
    fn drop(&mut self) {
        // SAFETY: both handles are live and owned by this `Control`; the child
        // is destroyed before its parent, as Win32 requires.
        unsafe {
            let _ = DestroyWindow(HWND(self.hwnd));
            let _ = DestroyWindow(HWND(self.host));
        }
    }
}
