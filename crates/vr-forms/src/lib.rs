#![forbid(unsafe_code)]
//! Form model, designer, codegen.
//!
//! For now this crate owns the `.vrform` source of truth only: the [`model`]
//! types, their JSON serialization, and [`ValidationError`] rules. The live
//! designer and the Dyon code generator build on top of this model.

pub mod designer;
mod error;
pub mod model;
mod validate;

#[cfg(test)]
mod tests;

pub use designer::{DesignerError, DesignerSurface, Handle, SurfaceEvent};
pub use error::{FormError, ValidationError};
pub use model::{Anchor, Bounds, Control, ControlKind, Dip, Form, Orientation, ScrollBars, Size};
