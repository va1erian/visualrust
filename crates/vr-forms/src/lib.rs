#![forbid(unsafe_code)]
//! Form model, designer, codegen.
//!
//! For now this crate owns the `.vrform` source of truth only: the [`model`]
//! types, their JSON serialization, and [`ValidationError`] rules. The live
//! designer and the Dyon code generator build on top of this model.
//!
//! The designer lives in [`design`]; it is built on the `xui` widget layer so
//! it can be hosted by the IDE shell's `xui_win32` app.

pub mod design;
mod error;
pub mod model;
mod validate;

#[cfg(test)]
mod tests;

pub use design::{DesignPoint, DesignerError, DesignerEvent, FormDesigner, Handle, PropertyValue};
pub use error::{FormError, ValidationError};
pub use model::{Anchor, Bounds, Control, ControlKind, Dip, Form, Orientation, ScrollBars, Size};
