//! Controls and their kind-specific properties.
//!
//! Every property that applies to all widgets (`name`, `bounds`, `text`,
//! `enabled`, `visible`, `tooltip`, `anchor`) lives on [`Control`]; the widget
//! family and its extra settings live in [`ControlKind`]. Modeling the kind as
//! an internally-tagged enum keeps the JSON a flat, readable object per
//! control while still rejecting unknown variants at parse time.

use serde::{Deserialize, Serialize};

use super::geometry::{Bounds, Dip};

/// One widget on a [`super::Form`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Control {
    /// The kind tag is emitted first so the file reads spec-first.
    pub kind: ControlKind,
    pub name: String,
    pub bounds: Bounds,
    /// Caption or initial text; empty for widgets that own no text.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub text: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_true")]
    pub visible: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tooltip: Option<String>,
    #[serde(default)]
    pub anchor: super::geometry::Anchor,
}

fn default_true() -> bool {
    true
}

/// Every win32ui widget family offered by the designer palette.
///
/// Variants that need configuration carry a dedicated property struct; the
/// plain widgets (`Button`, `Label`, `GroupBox`) are unit variants because all
/// of their state already lives on [`Control`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ControlKind {
    Button,
    Edit(EditProps),
    Label,
    CheckBox(CheckBoxProps),
    RadioGroup(ChoiceProps),
    ComboBox(ComboBoxProps),
    ListView(ListProps),
    TreeView(TreeViewProps),
    GroupBox,
    Panel(PanelProps),
    ScrollView(ScrollViewProps),
    Tabs(TabsProps),
    Menu(MenuProps),
    StatusBar(StatusBarProps),
    Toolbar(ToolbarProps),
    ProgressBar(RangeProps),
    Slider(SliderProps),
    GridView(ListProps),
    ColorPicker(ColorPickerProps),
    FlowText(FlowTextProps),
}

impl ControlKind {
    /// Stable tag matching the serialized enum name; used in error messages so
    /// validation never has to format a `Debug` value.
    pub fn tag(&self) -> &'static str {
        match self {
            ControlKind::Button => "button",
            ControlKind::Edit(_) => "edit",
            ControlKind::Label => "label",
            ControlKind::CheckBox(_) => "check_box",
            ControlKind::RadioGroup(_) => "radio_group",
            ControlKind::ComboBox(_) => "combo_box",
            ControlKind::ListView(_) => "list_view",
            ControlKind::TreeView(_) => "tree_view",
            ControlKind::GroupBox => "group_box",
            ControlKind::Panel(_) => "panel",
            ControlKind::ScrollView(_) => "scroll_view",
            ControlKind::Tabs(_) => "tabs",
            ControlKind::Menu(_) => "menu",
            ControlKind::StatusBar(_) => "status_bar",
            ControlKind::Toolbar(_) => "toolbar",
            ControlKind::ProgressBar(_) => "progress_bar",
            ControlKind::Slider(_) => "slider",
            ControlKind::GridView(_) => "grid_view",
            ControlKind::ColorPicker(_) => "color_picker",
            ControlKind::FlowText(_) => "flow_text",
        }
    }
}

/// Text entry settings.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EditProps {
    #[serde(default)]
    pub multiline: bool,
    #[serde(default)]
    pub password: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub placeholder: Option<String>,
}

/// Check state.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CheckBoxProps {
    #[serde(default)]
    pub checked: bool,
}

/// A fixed list of choices with the currently selected label.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChoiceProps {
    #[serde(default)]
    pub items: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected: Option<String>,
}

/// A drop-down list, optionally free-text editable.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComboBoxProps {
    #[serde(default)]
    pub items: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected: Option<String>,
    #[serde(default)]
    pub editable: bool,
}

/// A column in a `ListView` or `GridView`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Column {
    pub title: String,
    #[serde(default = "default_column_width")]
    pub width: Dip,
}

fn default_column_width() -> Dip {
    Dip::new(100.0)
}

/// Tabular settings shared by `ListView` and `GridView`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ListProps {
    #[serde(default)]
    pub columns: Vec<Column>,
    #[serde(default)]
    pub items: Vec<String>,
}

/// A node in a `TreeView`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TreeNode {
    pub label: String,
    #[serde(default)]
    pub children: Vec<TreeNode>,
}

/// Hierarchical tree settings.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TreeViewProps {
    #[serde(default)]
    pub nodes: Vec<TreeNode>,
}

/// Container drawing settings.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PanelProps {
    #[serde(default)]
    pub border: bool,
}

/// Which scroll bars a `ScrollView` shows.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScrollBars {
    #[default]
    None,
    Vertical,
    Horizontal,
    Both,
}

/// Scroll container settings.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScrollViewProps {
    #[serde(default)]
    pub bars: ScrollBars,
}

/// Tabbed pages.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TabsProps {
    #[serde(default)]
    pub tabs: Vec<String>,
    #[serde(default)]
    pub selected: u32,
}

/// A menu entry; `separator` items ignore `label`/`shortcut`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MenuItem {
    #[serde(default)]
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shortcut: Option<String>,
    #[serde(default)]
    pub separator: bool,
}

/// Menu bar settings.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MenuProps {
    #[serde(default)]
    pub items: Vec<MenuItem>,
}

/// Status bar settings.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StatusBarProps {
    #[serde(default)]
    pub parts: Vec<String>,
}

/// A toolbar button; `separator` items ignore `text`/`tooltip`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToolbarButton {
    #[serde(default)]
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tooltip: Option<String>,
    #[serde(default)]
    pub separator: bool,
}

/// Toolbar settings.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToolbarProps {
    #[serde(default)]
    pub buttons: Vec<ToolbarButton>,
}

/// Numeric range shared by progress bars and sliders.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RangeProps {
    #[serde(default)]
    pub min: f64,
    #[serde(default = "default_range_max")]
    pub max: f64,
    #[serde(default)]
    pub value: f64,
}

fn default_range_max() -> f64 {
    100.0
}

impl Default for RangeProps {
    fn default() -> Self {
        Self {
            min: 0.0,
            max: 100.0,
            value: 0.0,
        }
    }
}

/// Slider orientation.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Orientation {
    #[default]
    Horizontal,
    Vertical,
}

/// Slider settings.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SliderProps {
    #[serde(default)]
    pub min: f64,
    #[serde(default = "default_range_max")]
    pub max: f64,
    #[serde(default)]
    pub value: f64,
    #[serde(default)]
    pub orientation: Orientation,
}

impl Default for SliderProps {
    fn default() -> Self {
        Self {
            min: 0.0,
            max: 100.0,
            value: 0.0,
            orientation: Orientation::Horizontal,
        }
    }
}

/// Colour picker settings; `color` is `#rrggbb`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ColorPickerProps {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
}

/// Flowing text settings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FlowTextProps {
    #[serde(default = "default_true")]
    pub wrap: bool,
}

impl Default for FlowTextProps {
    fn default() -> Self {
        Self { wrap: true }
    }
}
