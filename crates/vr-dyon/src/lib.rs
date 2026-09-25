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
//! `ui_*` native functions let a Dyon program build and run a win32ui window:
//! `ui_window(title, width, height)` returns a window plan, `ui_label(text)`,
//! `ui_column(items)` and `ui_row(items)` return widget plans, and
//! `ui_run(window, root)` creates the widgets and blocks in the message loop.
//! Plans are Dyon custom objects (`Arc<Mutex<dyn Any>>`); the concrete widgets
//! are created inside `ui_run`, because win32ui only hands out its `Ui` there.
//! The bindings are documented in full in the [`ui`](crate::ui) module.

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
