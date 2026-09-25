//! C-compatible types shared with Scintilla's notification structures.
//!
//! The field order matches `vendor/scintilla/include/Scintilla.h` and
//! `ScintillaStructures.h`. On Windows x64 a `long` is 32-bit, so
//! [`SciPositionCR`] is `i32` while [`SciPosition`] (`ptrdiff_t`) is `isize`.
#![forbid(unsafe_code)]

use core::ffi::{c_char, c_void};

/// `Sci_Position`: a signed position, `ptrdiff_t`.
pub type SciPosition = isize;
/// `Sci_PositionCR`: a position compatible with Win32 `CHARRANGE`, a C `long`.
pub type SciPositionCR = i32;
/// `uptr_t`: an unsigned integer wide enough to hold a pointer.
pub type UptrT = usize;
/// `sptr_t`: a signed integer wide enough to hold a pointer.
pub type SptrT = isize;

/// `Sci_NotifyHeader`, layout-compatible with Win32 `NMHDR`.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SciNotifyHeader {
    pub hwnd_from: *mut c_void,
    pub id_from: UptrT,
    pub code: u32,
}

/// `SCNotification`: the payload of a `WM_NOTIFY` from a Scintilla control.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ScNotification {
    pub nmhdr: SciNotifyHeader,
    pub position: SciPosition,
    pub ch: i32,
    pub modifiers: i32,
    pub modification_type: i32,
    pub text: *const c_char,
    pub length: SciPosition,
    pub lines_added: SciPosition,
    pub message: i32,
    pub w_param: UptrT,
    pub l_param: SptrT,
    pub line: SciPosition,
    pub fold_level_now: i32,
    pub fold_level_prev: i32,
    pub margin: i32,
    pub list_type: i32,
    pub x: i32,
    pub y: i32,
    pub token: i32,
    pub annotation_lines_added: SciPosition,
    pub updated: i32,
    pub list_completion_method: i32,
    pub character_source: i32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(all(target_os = "windows", target_pointer_width = "64"))]
    fn notification_layout_matches_the_c_header() {
        assert_eq!(size_of::<SciNotifyHeader>(), 24);
        assert_eq!(size_of::<ScNotification>(), 160);
    }
}
