#![forbid(unsafe_code)]
//! Form model, designer, codegen.
//!
//! For now this crate owns the `.vrform` source of truth only: the [`model`]
//! types, their JSON serialization, and [`ValidationError`] rules. The live
//! designer lives in [`design`] and turns the model into an editable widget;
//! [`codegen`] turns it into Dyon source. Both are built on the `xui` widget
//! layer so they can be hosted by the IDE shell's `xui_win32` app.

pub mod codegen;
pub mod design;
mod error;
pub mod model;
mod validate;

#[cfg(test)]
mod tests;

pub use codegen::{CodegenOptions, EmitHandlers, Indent, generate};
pub use design::{DesignPoint, DesignerError, DesignerEvent, FormDesigner, Handle, PropertyValue};
pub use error::{FormError, ValidationError};
pub use model::{
    Anchor, Bounds, Control, ControlKind, Dip, Form, Orientation, ScrollBars, Size, anchored,
    apply_anchors,
};
