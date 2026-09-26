//! Capturing a live window to an in-memory image or a PNG file.
//!
//! The composited backend (`Windows.Graphics.Capture`, win32ui's `wgc`) is
//! tried first because it includes the DWM frame and never raises, focuses or
//! unoccludes the target. If it is unavailable, the `PrintWindow` fallback
//! renders the same window without COM or DWM.

use std::fmt;
use std::path::{Path, PathBuf};

use win32ui::{Hwnd, RgbaImage};

use crate::error::{Result, ToolingError};

/// How to pick the window to capture.
#[derive(Clone, Debug)]
pub enum WindowSelector {
    /// A known handle.
    Hwnd(Hwnd),
    /// A case-insensitive substring of the window title.
    Title(String),
    /// The id of the process that owns the window.
    Pid(u32),
}

impl WindowSelector {
    /// Selects a window by handle.
    pub fn hwnd(hwnd: Hwnd) -> WindowSelector {
        WindowSelector::Hwnd(hwnd)
    }

    /// Selects a window whose title contains `needle`, case-insensitively.
    pub fn title(needle: impl Into<String>) -> WindowSelector {
        WindowSelector::Title(needle.into())
    }

    /// Selects a window owned by process `pid`.
    pub fn pid(pid: u32) -> WindowSelector {
        WindowSelector::Pid(pid)
    }
}

impl fmt::Display for WindowSelector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WindowSelector::Hwnd(hwnd) => write!(f, "hwnd 0x{:x}", hwnd.raw()),
            WindowSelector::Title(title) => write!(f, "title containing {title:?}"),
            WindowSelector::Pid(pid) => write!(f, "pid {pid}"),
        }
    }
}

/// Captures the selected window into a straight-RGBA image.
///
/// Tries the composited backend first and falls back to `PrintWindow`; the
/// images differ in size because the composited region is the visible DWM frame
/// while `PrintWindow` includes the invisible resize border.
pub fn capture_image(selector: &WindowSelector) -> Result<RgbaImage> {
    let hwnd = resolve(selector)?;
    // win32ui does not initialise COM; the composited capture wants an STA on
    // this thread. The guard is inert when COM is already up.
    let _apartment = crate::sys::com::Apartment::initialize();
    match win32ui::capture::capture_hwnd(hwnd) {
        Ok(image) => Ok(image),
        Err(error) => crate::sys::print_window(hwnd).ok_or_else(|| ToolingError::CaptureFailed {
            selector: selector.to_string(),
            reason: error.to_string(),
        }),
    }
}

/// Captures the selected window and writes it as a PNG to `out_path`.
///
/// Creates any missing parent directory and returns the path written.
pub fn capture_window(selector: WindowSelector, out_path: impl AsRef<Path>) -> Result<PathBuf> {
    let image = capture_image(&selector)?;
    let path = out_path.as_ref().to_path_buf();
    crate::golden::write_png(&image, &path)?;
    Ok(path)
}

/// Captures the selected window with the `PrintWindow` renderer only.
///
/// The composited backend is preferred for its DWM frame, but the pinned win32ui
/// rev faults inside `Windows.Graphics.Capture` when the target belongs to
/// another process, and a fault cannot be caught to reach the fallback. A caller
/// that has launched the target itself (an exported app under test) uses this to
/// get a rendering without risking the harness.
pub fn capture_window_rendered(
    selector: WindowSelector,
    out_path: impl AsRef<Path>,
) -> Result<PathBuf> {
    let hwnd = resolve(&selector)?;
    let image = crate::sys::print_window(hwnd).ok_or_else(|| ToolingError::CaptureFailed {
        selector: selector.to_string(),
        reason: "PrintWindow returned no image".to_owned(),
    })?;
    let path = out_path.as_ref().to_path_buf();
    crate::golden::write_png(&image, &path)?;
    Ok(path)
}

/// Resolves a selector to a live handle, erroring with a typed reason.
fn resolve(selector: &WindowSelector) -> Result<Hwnd> {
    match selector {
        WindowSelector::Hwnd(hwnd) if hwnd.is_alive() => Ok(*hwnd),
        WindowSelector::Hwnd(hwnd) => Err(ToolingError::WindowGone { hwnd: hwnd.raw() }),
        WindowSelector::Title(needle) => {
            let needle = needle.to_lowercase();
            crate::sys::find_window(|_, title| title.to_lowercase().contains(&needle))
                .ok_or_else(|| not_found(selector))
        }
        WindowSelector::Pid(pid) => crate::sys::find_window(|candidate, _| candidate == *pid)
            .ok_or_else(|| not_found(selector)),
    }
}

fn not_found(selector: &WindowSelector) -> ToolingError {
    ToolingError::WindowNotFound {
        selector: selector.to_string(),
    }
}
