//! Raw FFI for the vendored, pinned Scintilla 5.5.x Windows build.
//!
//! The C++ sources live in `vendor/scintilla` (see `VENDOR.md` for the tag and
//! update procedure) and are compiled into a static library by `build.rs`, so
//! building this crate needs neither git submodules nor network access. This
//! crate deliberately exposes only the curated subset of the Scintilla API the
//! editor needs; it is not generated with `bindgen`.
//!
//! Calling anything here is `unsafe`: the caller must own a live Scintilla
//! `HWND` and uphold Scintilla's pointer and length contracts for `wParam` /
//! `lParam`. The safe wrapper lives in `vr-scintilla`.
//!
//! Scintilla is distributed under the Historical Permission Notice and
//! Disclaimer (HPND) licence; the full text is in `LICENSE-SCINTILLA` and in
//! `vendor/scintilla/License.txt`.

pub mod messages;
pub mod notifications;
pub mod structures;
pub mod sys;

pub use messages::*;
pub use notifications::*;
pub use structures::{ScNotification, SciNotifyHeader};
pub use sys::{Scintilla_RegisterClasses, Scintilla_ReleaseResources, send_message};
