//! Subclassing the Scintilla control's parent to receive its notifications.
//!
//! Scintilla is a foreign Win32 class: `NotifyParent` sends a `WM_NOTIFY` to
//! the control's *immediate parent*, not to the control itself. The win32ui
//! widget layer owns that parent (a `Custom`'s child `HWND`) and does not route
//! `WM_NOTIFY`, so this module chains a subclass in front of its window
//! procedure with `SetWindowSubclass`, decodes each `SCNotification`, and hands
//! the safe [`Scn`](crate::Scn) to a [`NotificationSink`] implemented above
//! `sys`. Only `DefSubclassProc` and the raw pointer cast are `unsafe`.
//!
//! All `unsafe` in the crate lives in `sys`; every block below carries a
//! `// SAFETY:` note.

use core::ffi::c_void;

use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass};
use windows::Win32::UI::WindowsAndMessaging::{GetParent, WM_NOTIFY};

use vr_scintilla_sys::{ScNotification, SciNotifyHeader};

use crate::scn::Scn;

use super::notification::decode_notification;

/// A stable id for this crate's one subclass on a parent window. Comctl32 keys
/// subclasses by `(proc, id)`, so it only has to be unique per window.
const SUBCLASS_ID: usize = 0x7672_7363; // "vrsc"

/// The safe half of a Scintilla notification: something (the host widget) that
/// turns a decoded [`Scn`] into application state.
pub(crate) trait NotificationSink {
    /// Handles one decoded notification. Called on the thread that owns the
    /// control, from inside the parent's `WM_NOTIFY`.
    fn notification(&self, scn: Scn);
}

/// The heap state a subclass instance points at: the control whose
/// notifications are wanted, plus the sink that handles them.
struct SubclassData {
    control: *mut c_void,
    sink: Box<dyn NotificationSink>,
}

/// An installed parent subclass; dropping it removes the subclass and frees
/// its [`SubclassData`].
pub(crate) struct ParentSubclass {
    parent: *mut c_void,
    data: *mut SubclassData,
}

impl ParentSubclass {
    /// Subclasses the immediate parent of `control` and routes only that
    /// control's `WM_NOTIFY` messages to `sink`. Returns `None` when the
    /// parent handle cannot be resolved or the subclass cannot be installed.
    pub(crate) fn install(
        control: *mut c_void,
        sink: Box<dyn NotificationSink>,
    ) -> Option<ParentSubclass> {
        if control.is_null() {
            return None;
        }
        // SAFETY: the caller passes a live control handle; `GetParent` only
        // reads, and a null result is rejected.
        let Ok(parent) = (unsafe { GetParent(HWND(control)) }) else {
            return None;
        };
        if parent.0.is_null() {
            return None;
        }
        let data = Box::into_raw(Box::new(SubclassData { control, sink }));
        // SAFETY: `parent` is live and `data` outlives the subclass (freed in
        // `Drop` after `RemoveWindowSubclass`); the callback keeps no other
        // state.
        let installed =
            unsafe { SetWindowSubclass(parent, Some(notify_proc), SUBCLASS_ID, data as usize) };
        if !installed.as_bool() {
            // SAFETY: `data` came from `Box::into_raw` above and was never
            // handed to comctl32 because the install failed.
            unsafe {
                drop(Box::from_raw(data));
            }
            return None;
        }
        Some(ParentSubclass {
            parent: parent.0,
            data,
        })
    }
}

impl Drop for ParentSubclass {
    fn drop(&mut self) {
        // SAFETY: `self.parent` is the window the subclass was installed on
        // and `notify_proc`/`SUBCLASS_ID` identify it. Removing it first means
        // no further callback can reach `self.data`.
        unsafe {
            let _ = RemoveWindowSubclass(HWND(self.parent), Some(notify_proc), SUBCLASS_ID);
            drop(Box::from_raw(self.data));
        }
    }
}

/// The subclass procedure. It decodes notifications from the hosted control and
/// forwards everything else to the window's original procedure.
///
/// # Safety
///
/// Called by comctl32 for the subclass installed by [`ParentSubclass::install`].
/// `refdata` is the `SubclassData` pointer it installed, valid until `Drop`.
unsafe extern "system" fn notify_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _id: usize,
    refdata: usize,
) -> LRESULT {
    if msg == WM_NOTIFY && refdata != 0 && lparam.0 != 0 {
        // SAFETY: `refdata` is the `SubclassData` installed above, and the
        // subclass is removed only after it is freed.
        let data = unsafe { &*(refdata as *const SubclassData) };
        // SAFETY: for `WM_NOTIFY` `lParam` points at an `NMHDR`; an
        // `SCNotification` starts with one, so reading `hwnd_from` through that
        // layout is valid and only identifies the sender.
        let header = unsafe { &*(lparam.0 as *const SciNotifyHeader) };
        if header.hwnd_from == data.control {
            // SAFETY: the sender is the live control and Scintilla guarantees
            // the `SCNotification` is valid for this dispatch; the decoded
            // `Scn` copies any text it needs.
            let scn = unsafe { decode_notification(lparam.0 as *const ScNotification) };
            // A panic unwinding across this `extern "system"` boundary would be
            // undefined behaviour, so isolate it rather than let it propagate.
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                data.sink.notification(scn);
            }));
            return LRESULT(0);
        }
    }
    // SAFETY: `hwnd` is the window this subclass was installed on; forwarding
    // preserves the widget layer's own handling of every other message.
    unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) }
}
