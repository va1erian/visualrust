//! Dyon runtime integration for VisualRust.
//!
//! [`DyonRuntime`] wraps a compiled [`dyon::Module`] and its
//! [`dyon::Runtime`], loads source from a string or a file, and surfaces
//! failures as the typed [`DyonError`]. Native functions live in
//! [`native`](crate::native) and are registered before source is compiled so
//! Dyon's lifetime checker sees their signatures.
//!
//! # UI bindings
//!
//! `ui_*` native functions let a Dyon program build and run a xui window:
//! `ui_window(title, width, height)` returns a window plan,
//! `ui_free(width, height)` returns an anchored root, the `ui_<kind>(text, x,
//! y, w, h)` constructors return widget plans, and
//! `ui_run(window, root)` registers the tree and returns. The host loop starts
//! from [`DyonRuntime::run`] once `main` has returned, so handlers run with the
//! Dyon program off the stack and can never re-enter it. The bindings are
//! documented in full in the crate's `ui` module.

#![forbid(unsafe_code)]

pub mod error;
mod native;
mod runtime;
#[cfg(feature = "ui")]
mod ui;

pub use error::{DyonError, SourcePosition};
pub use runtime::DyonRuntime;

pub use dyon::Variable;
pub use dyon::embed::PushVariable;
