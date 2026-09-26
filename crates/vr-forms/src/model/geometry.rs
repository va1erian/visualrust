//! Design units and layout primitives shared by every control.
//!
//! These are deliberately independent of `xui`: a `.vrform` is persisted
//! before any window exists and a form may be edited on a machine whose DPI
//! differs from the one it was authored on. `Dip` is a device-independent
//! design unit the designer scales at render time.

use serde::{Deserialize, Serialize};

/// A device-independent design unit. Stored as a `f64` so fractional scaling
/// survives a round trip; validation rejects non-finite values.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Dip(pub f64);

impl Dip {
    pub const ZERO: Self = Self(0.0);

    pub const fn new(value: f64) -> Self {
        Self(value)
    }

    pub const fn get(self) -> f64 {
        self.0
    }

    /// A dimension is usable only when positive and finite; `NaN` compares
    /// false against everything and would silently corrupt layout maths.
    pub fn is_positive(self) -> bool {
        self.0.is_finite() && self.0 > 0.0
    }

    pub fn is_non_negative(self) -> bool {
        self.0.is_finite() && self.0 >= 0.0
    }
}

impl From<f64> for Dip {
    fn from(value: f64) -> Self {
        Self(value)
    }
}

impl From<Dip> for f64 {
    fn from(value: Dip) -> Self {
        value.0
    }
}

impl std::fmt::Display for Dip {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// The design surface of a form.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Size {
    pub width: Dip,
    pub height: Dip,
}

/// An axis-aligned rectangle in design units, relative to the form origin.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bounds {
    pub x: Dip,
    pub y: Dip,
    pub width: Dip,
    pub height: Dip,
}

/// Where a control sticks when its parent is resized.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Anchor {
    #[default]
    TopLeft,
    Top,
    TopRight,
    Left,
    Center,
    Right,
    BottomLeft,
    Bottom,
    BottomRight,
}
