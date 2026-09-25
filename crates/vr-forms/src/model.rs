//! `.vrform`: the persisted, designer-owned description of a form.
//!
//! ## Format
//!
//! The file is **JSON**. A form is a heterogeneous, ordered list of controls
//! whose per-kind settings differ; JSON carries that shape directly, keeps the
//! control order as authored (z-order), and lets [`ControlKind`] use serde's
//! internally-tagged enum so unknown widget types are a parse error. TOML's
//! value-before-table ordering makes per-control nested enum tables brittle,
//! and non-UTF-8-safe name rules matter less here than a stable round trip.
//! Field order is declaration order, so re-serializing is byte-stable.

pub mod control;
pub mod geometry;

pub use control::{
    CheckBoxProps, ChoiceProps, ColorPickerProps, Column, ComboBoxProps, Control, ControlKind,
    EditProps, FlowTextProps, ListProps, MenuItem, MenuProps, Orientation, PanelProps, RangeProps,
    ScrollBars, ScrollViewProps, SliderProps, StatusBarProps, TabsProps, ToolbarButton,
    ToolbarProps, TreeNode, TreeViewProps,
};
pub use geometry::{Anchor, Bounds, Dip, Size};

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::{FormError, ValidationError};

/// A designable form: its identity, design surface and ordered controls.
///
/// `controls` order is z-order: later controls paint on top. Validation treats
/// the list as a namespace, so control names must be unique.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Form {
    pub name: String,
    pub size: Size,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub controls: Vec<Control>,
}

impl Form {
    /// Parses and validates form text.
    pub fn from_json(input: &str) -> Result<Self, FormError> {
        let form: Self = serde_json::from_str(input)?;
        form.validate()?;
        Ok(form)
    }

    /// Validates then serializes to pretty JSON.
    pub fn to_json(&self) -> Result<String, FormError> {
        self.validate()?;
        serde_json::to_string_pretty(self).map_err(FormError::Serialize)
    }

    /// Reads, parses and validates a `.vrform` from disk.
    pub fn load(path: impl AsRef<Path>) -> Result<Self, FormError> {
        let path = path.as_ref();
        let text = std::fs::read_to_string(path).map_err(|source| FormError::Read {
            path: path.to_path_buf(),
            source,
        })?;
        Self::from_json(&text)
    }

    /// Validates then writes the form to disk.
    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), FormError> {
        let path = path.as_ref();
        let text = self.to_json()?;
        std::fs::write(path, text).map_err(|source| FormError::Write {
            path: path.to_path_buf(),
            source,
        })
    }

    /// Checks the form against every structural rule.
    pub fn validate(&self) -> Result<(), ValidationError> {
        crate::validate::validate(self)
    }
}
