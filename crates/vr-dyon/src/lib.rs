//! Dyon runtime integration for VisualRust.
//!
//! [`DyonRuntime`] wraps a compiled [`dyon::Module`] and its
//! [`dyon::Runtime`], loads source from a string or a file, and surfaces
//! failures as the typed [`DyonError`]. Native functions live in
//! [`native`](crate::native) and are registered before source is compiled so
//! Dyon's lifetime checker sees their signatures.

#![forbid(unsafe_code)]

pub mod error;
mod native;
mod runtime;

pub use error::{DyonError, SourcePosition};
pub use runtime::DyonRuntime;

pub use dyon::Variable;
pub use dyon::embed::PushVariable;
