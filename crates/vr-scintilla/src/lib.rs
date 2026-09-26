//! Safe Rust wrapper over the vendored Scintilla control.
//!
//! Everything the rest of the IDE needs from Scintilla goes through typed
//! methods on [`Scintilla`] and the [`Scn`] notification enum, so callers never
//! build a raw `SCI_*` message by hand. [`ScintillaHost`] embeds a control in a
//! xui `Custom`, fills the pane on resize, routes `SCN_*` notifications and
//! applies Dyon highlighting from `vr-syntax`.
//!
//! `unsafe` lives only in the private [`sys`] module, which wraps every raw
//! pointer and `HWND` use in a safe method with a documented contract. The crate
//! root therefore `deny`s `unsafe_code`; a crate-level `forbid` cannot be
//! relaxed inside `sys`, which is why this is `deny` rather than `forbid` (the
//! same pattern `vr-tooling` uses).
//!
//! ```no_run
//! use vr_scintilla::{Color, Scintilla};
//!
//! # fn main() -> vr_scintilla::Result<()> {
//! let editor = Scintilla::new()?;
//! editor.set_text("fn main() {}");
//! editor.set_style_fore(0, Color::rgb(0x20, 0x80, 0x20));
//! assert_eq!(editor.text()?, "fn main() {}");
//! # Ok(())
//! # }
//! ```

#![deny(unsafe_code)]

mod codec;
mod error;
mod handle;
mod host;
mod messages;
mod scn;
mod sys;
mod theme_map;
mod types;

pub use error::{Error, Result};
pub use handle::Scintilla;
pub use host::ScintillaHost;
pub use scn::Scn;
pub use types::{AnnotationVisible, Color, IndicatorStyle, MarginType, MarkerSymbol};

/// Decodes the `SCNotification` behind a `WM_NOTIFY` `lParam`.
///
/// # Safety
///
/// `ptr` must point to a live `SCNotification`. When its `text` member is
/// non-null it must be a NUL-terminated string; Scintilla only guarantees that
/// for the duration of the `WM_NOTIFY` dispatch, so the returned [`Scn`] owns a
/// copy of any text it needs.
pub use sys::decode_notification;
