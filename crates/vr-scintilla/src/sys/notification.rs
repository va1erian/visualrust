//! Dereferences a raw `SCNotification` into the safe [`Scn`] model.
//!
//! The only unsafe part is reading the struct and its optional text pointer;
//! the mapping itself lives in `crate::scn`.

use core::ffi::CStr;

use vr_scintilla_sys::ScNotification;

use crate::scn::Scn;

/// Decodes the `SCNotification` at `ptr`.
///
/// # Safety
///
/// `ptr` must point to a live, correctly aligned `SCNotification`. When its
/// `text` member is non-null it must be a NUL-terminated string valid for this
/// call. Scintilla guarantees both only for the duration of the `WM_NOTIFY`
/// dispatch; the returned [`Scn`] copies any text it needs.
pub unsafe fn decode_notification(ptr: *const ScNotification) -> Scn {
    // SAFETY: the caller guarantees `ptr` is a live, aligned `ScNotification`;
    // `ScNotification` is `Copy`, so this is a plain read.
    let notification = unsafe { *ptr };
    let text = if notification.text.is_null() {
        None
    } else {
        // SAFETY: the function contract makes non-null `text` a NUL-terminated
        // string valid for this call.
        Some(
            unsafe { CStr::from_ptr(notification.text) }
                .to_string_lossy()
                .into_owned(),
        )
    };
    Scn::from_notification(&notification, text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use vr_scintilla_sys::notifications::{
        SCN_CHARADDED, SCN_MODIFIED, SCN_STYLENEEDED, SCN_USERLISTSELECTION,
    };

    /// A synthetic notification with all raw fields cleared, standing in for
    /// the uninitialised bytes a `WM_NOTIFY` would carry.
    fn empty() -> ScNotification {
        // SAFETY: `SCNotification` is a plain-old-data struct of integers and
        // raw pointers, all of which are valid when zeroed.
        unsafe { core::mem::zeroed() }
    }

    #[test]
    fn decodes_style_needed_from_a_synthetic_buffer() {
        let mut notification = empty();
        notification.nmhdr.code = SCN_STYLENEEDED;
        notification.position = 42;
        // SAFETY: `notification` is initialised and live for the call.
        let decoded = unsafe { decode_notification(&notification) };
        assert_eq!(decoded, Scn::StyleNeeded { position: 42 });
    }

    #[test]
    fn decodes_char_added_from_a_synthetic_buffer() {
        let mut notification = empty();
        notification.nmhdr.code = SCN_CHARADDED;
        notification.ch = 'x' as i32;
        // SAFETY: `notification` is initialised and live for the call.
        let decoded = unsafe { decode_notification(&notification) };
        assert_eq!(decoded, Scn::CharAdded { ch: 'x' });
    }

    #[test]
    fn decodes_modified_text_from_a_synthetic_buffer() {
        let mut notification = empty();
        notification.nmhdr.code = SCN_MODIFIED;
        notification.position = 3;
        notification.length = 5;
        notification.lines_added = 1;
        // `c"hello"` is a `'static` buffer, so the pointer stays valid.
        notification.text = c"hello".as_ptr();
        // SAFETY: `notification` is initialised and its text pointer is a live
        // NUL-terminated buffer.
        let decoded = unsafe { decode_notification(&notification) };
        assert_eq!(
            decoded,
            Scn::Modified {
                position: 3,
                modification_type: 0,
                text: Some("hello".to_owned()),
                length: 5,
                lines_added: 1,
                message: 0,
            }
        );
    }

    #[test]
    fn decodes_unknown_codes_as_other() {
        let mut notification = empty();
        notification.nmhdr.code = SCN_USERLISTSELECTION;
        // SAFETY: `notification` is initialised and live for the call.
        let decoded = unsafe { decode_notification(&notification) };
        assert_eq!(
            decoded,
            Scn::Other {
                code: SCN_USERLISTSELECTION
            }
        );
    }
}
