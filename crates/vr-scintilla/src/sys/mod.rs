//! The only module permitted to use `unsafe`.
//!
//! It wraps the raw `vr-scintilla-sys` FFI and the Win32 window calls behind
//! safe methods whose contracts are documented. Every `unsafe` block carries a
//! `// SAFETY:` note. Callers above this module see only Rust types; the
//! control's `HWND` never escapes.

#![allow(unsafe_code)]

mod notification;
mod window;

pub use notification::decode_notification;
pub(crate) use window::Control;
