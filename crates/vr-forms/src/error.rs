//! Typed failures for loading, writing and validating a `.vrform`.

use std::io;
use std::path::PathBuf;

/// Anything that can go wrong turning `.vrform` bytes into a valid form.
#[derive(Debug, thiserror::Error)]
pub enum FormError {
    #[error("could not read form `{}`: {source}", .path.display())]
    Read {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("could not write form `{}`: {source}", .path.display())]
    Write {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("form is not valid JSON: {0}")]
    Parse(#[from] serde_json::Error),
    /// Separate from [`FormError::Parse`] because `serde_json::Error` can only
    /// back one `From` impl; serialization failures are mapped explicitly.
    #[error("form could not be serialized: {0}")]
    Serialize(serde_json::Error),
    #[error(transparent)]
    Invalid(#[from] ValidationError),
}

/// A form that parsed but describes an unusable designer surface.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum ValidationError {
    #[error("form name must not be empty")]
    EmptyFormName,

    #[error("form name `{name}` is not a valid name; use letters, digits, '-' or '_'")]
    InvalidFormName { name: String },

    #[error("form size must be positive and finite, got {width} x {height}")]
    InvalidFormSize { width: f64, height: f64 },

    #[error("control at index {index} must have a name")]
    EmptyControlName { index: usize },

    #[error("control name `{name}` is not a valid name; use letters, digits, '-' or '_'")]
    InvalidControlName { name: String },

    #[error("duplicate control name `{name}`")]
    DuplicateControlName { name: String },

    #[error("control `{name}` ({kind}) must have positive, finite width and height")]
    InvalidControlSize { name: String, kind: &'static str },

    #[error(
        "control `{name}` ({kind}) at ({x}, {y}) size {width} x {height} is not inside the \
         form ({form_width} x {form_height})"
    )]
    ControlOutOfBounds {
        name: String,
        kind: &'static str,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        form_width: f64,
        form_height: f64,
    },

    #[error("control `{name}` ({kind}) has an invalid range: min {min}, max {max}, value {value}")]
    InvalidRange {
        name: String,
        kind: &'static str,
        min: f64,
        max: f64,
        value: f64,
    },

    #[error("control `{name}` ({kind}) selects index {selected}, but only {len} item(s) exist")]
    SelectionOutOfRange {
        name: String,
        kind: &'static str,
        selected: u32,
        len: usize,
    },

    #[error("control `{name}` ({kind}) selects `{value}`, which is not one of its items")]
    UnknownSelection {
        name: String,
        kind: &'static str,
        value: String,
    },

    #[error("control `{name}` ({kind}) requires at least one item")]
    NoItems { name: String, kind: &'static str },

    #[error("control `{name}` has invalid color `{value}`; expected #rrggbb")]
    InvalidColor { name: String, value: String },
}
