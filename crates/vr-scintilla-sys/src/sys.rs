//! The raw, `unsafe` FFI surface.
//!
//! This is the only module in the crate permitted to contain `unsafe`; every
//! block carries a `// SAFETY:` note. Callers must guarantee that any window
//! handle passed here is a live Scintilla control and that `wParam` / `lParam`
//! satisfy the contract of the `SCI_*` message being sent.

use core::ffi::c_void;

unsafe extern "C" {
    /// Registers the `Scintilla` and call-tip window classes and initialises
    /// the platform layer. `hinstance` is the module instance that owns the
    /// classes (the executable's `HINSTANCE` for a static build).
    ///
    /// Returns non-zero on success. Must be called once before creating a
    /// Scintilla control.
    pub fn Scintilla_RegisterClasses(hinstance: *mut c_void) -> i32;

    /// Unregisters the classes and releases platform resources. Call once, at
    /// shutdown, after all Scintilla controls are destroyed.
    ///
    /// Returns non-zero on success.
    pub fn Scintilla_ReleaseResources() -> i32;
}

unsafe extern "system" {
    fn SendMessageW(hwnd: *mut c_void, message: u32, wparam: usize, lparam: isize) -> isize;
}

/// Sends a Scintilla message (`SCI_*`) to the control at `hwnd`.
///
/// # Safety
///
/// `hwnd` must be a live `Scintilla` window owned by the caller. `wparam` and
/// `lparam` must match the contract of `message`; Scintilla dereferences
/// `lparam` as a pointer for messages that carry one.
pub unsafe fn send_message(hwnd: *mut c_void, message: u32, wparam: usize, lparam: isize) -> isize {
    // SAFETY: the caller upholds the window and parameter contracts; the
    // Scintilla window procedure handles the message synchronously.
    unsafe { SendMessageW(hwnd, message, wparam, lparam) }
}
