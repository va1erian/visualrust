//! Per-control line emission and the helper vocabulary [`super::generate`]
//! builds on. Split out so the public API and its long contract doc stay in
//! one small file.

use std::collections::HashSet;
use std::fmt::Write as _;

use super::escape::quoted;
use crate::model::{
    Anchor, ChoiceProps, ComboBoxProps, Control, ControlKind, ListProps, MenuProps, Orientation,
    RangeProps, ScrollBars, SliderProps, StatusBarProps, TabsProps, ToolbarProps, TreeNode,
    TreeViewProps,
};

/// Writes the constructor, `ui_add`, setters, anchor and event wiring for one
/// control, registering any handler names it references.
pub(super) fn emit_control(
    out: &mut String,
    control: &Control,
    index: usize,
    unit: &str,
    handlers: &mut Handlers,
) {
    let var = format!("c{index}");
    let bounds = control.bounds;
    let caption = if passes_caption(&control.kind) {
        control.text.as_str()
    } else {
        ""
    };

    let _ = writeln!(
        out,
        "{unit}{var} := {}({}, {}, {}, {}, {})",
        constructor(&control.kind),
        quoted(caption),
        number(bounds.x.get()),
        number(bounds.y.get()),
        number(bounds.width.get()),
        number(bounds.height.get())
    );
    let _ = writeln!(out, "{unit}ui_add(root, {var})");

    if !control.text.is_empty() && !passes_caption(&control.kind) {
        let _ = writeln!(out, "{unit}ui_set_text({var}, {})", quoted(&control.text));
    }
    if let Some(tooltip) = &control.tooltip {
        let _ = writeln!(out, "{unit}ui_set_tooltip({var}, {})", quoted(tooltip));
    }
    if !control.enabled {
        let _ = writeln!(out, "{unit}ui_set_enabled({var}, false)");
    }
    if !control.visible {
        let _ = writeln!(out, "{unit}ui_set_visible({var}, false)");
    }
    for line in kind_setters(&control.kind, &var) {
        let _ = writeln!(out, "{unit}{line}");
    }

    let _ = writeln!(
        out,
        "{unit}ui_anchor({var}, {})",
        quoted(anchor_name(control.anchor))
    );
    for event in events(&control.kind) {
        let handler = handlers.register(event, &control.name, index);
        let _ = writeln!(
            out,
            "{unit}ui_on({var}, {}, {})",
            quoted(event),
            quoted(&handler)
        );
    }
}

/// The constructor name for a kind.
fn constructor(kind: &ControlKind) -> &'static str {
    match kind {
        ControlKind::Button => "ui_button",
        ControlKind::Edit(_) => "ui_edit",
        ControlKind::Label => "ui_label",
        ControlKind::CheckBox(_) => "ui_check_box",
        ControlKind::RadioGroup(_) => "ui_radio_group",
        ControlKind::ComboBox(_) => "ui_combo_box",
        ControlKind::ListView(_) => "ui_list_view",
        ControlKind::TreeView(_) => "ui_tree_view",
        ControlKind::GroupBox => "ui_group_box",
        ControlKind::Panel(_) => "ui_panel",
        ControlKind::ScrollView(_) => "ui_scroll_view",
        ControlKind::Tabs(_) => "ui_tabs",
        ControlKind::Menu(_) => "ui_menu",
        ControlKind::StatusBar(_) => "ui_status_bar",
        ControlKind::Toolbar(_) => "ui_toolbar",
        ControlKind::ProgressBar(_) => "ui_progress_bar",
        ControlKind::Slider(_) => "ui_slider",
        ControlKind::GridView(_) => "ui_grid_view",
        ControlKind::ColorPicker(_) => "ui_color_picker",
        ControlKind::FlowText(_) => "ui_flow_text",
    }
}

/// Whether the model text is a caption the constructor takes; every other kind
/// gets its text (if any) through `ui_set_text`.
fn passes_caption(kind: &ControlKind) -> bool {
    matches!(
        kind,
        ControlKind::Button
            | ControlKind::Label
            | ControlKind::CheckBox(_)
            | ControlKind::GroupBox
            | ControlKind::RadioGroup(_)
            | ControlKind::ComboBox(_)
    )
}

/// The kind-specific setters for one control, in a fixed order.
fn kind_setters(kind: &ControlKind, var: &str) -> Vec<String> {
    let mut lines = Vec::new();
    match kind {
        ControlKind::Edit(props) => {
            if props.multiline {
                lines.push(format!("ui_set_multiline({var}, true)"));
            }
            if props.password {
                lines.push(format!("ui_set_password({var}, true)"));
            }
            if let Some(placeholder) = &props.placeholder {
                lines.push(format!(
                    "ui_set_placeholder({var}, {})",
                    quoted(placeholder)
                ));
            }
        }
        ControlKind::CheckBox(props) => {
            lines.push(format!("ui_set_checked({var}, {})", props.checked));
        }
        ControlKind::RadioGroup(props) => choice_setters(&mut lines, var, props),
        ControlKind::ComboBox(props) => combo_setters(&mut lines, var, props),
        ControlKind::ListView(props) | ControlKind::GridView(props) => {
            list_setters(&mut lines, var, props);
        }
        ControlKind::TreeView(props) => tree_setters(&mut lines, var, props),
        ControlKind::Panel(props) => {
            if props.border {
                lines.push(format!("ui_set_border({var}, true)"));
            }
        }
        ControlKind::ScrollView(props) => {
            lines.push(format!(
                "ui_set_scroll_bars({var}, {})",
                quoted(scroll_bars(props.bars))
            ));
        }
        ControlKind::Tabs(props) => tabs_setters(&mut lines, var, props),
        ControlKind::Menu(props) => menu_setters(&mut lines, var, props),
        ControlKind::StatusBar(props) => status_bar_setters(&mut lines, var, props),
        ControlKind::Toolbar(props) => toolbar_setters(&mut lines, var, props),
        ControlKind::ProgressBar(props) => range_setters(&mut lines, var, props),
        ControlKind::Slider(props) => slider_setters(&mut lines, var, props),
        ControlKind::ColorPicker(props) => {
            if let Some(color) = &props.color {
                lines.push(format!("ui_set_color({var}, {})", quoted(color)));
            }
        }
        ControlKind::FlowText(props) => {
            lines.push(format!("ui_set_wrap({var}, {})", props.wrap));
        }
        ControlKind::Button | ControlKind::Label | ControlKind::GroupBox => {}
    }
    lines
}

fn choice_setters(lines: &mut Vec<String>, var: &str, props: &ChoiceProps) {
    items_setter(lines, var, &props.items);
    if let Some(selected) = &props.selected {
        lines.push(format!("ui_set_selected({var}, {})", quoted(selected)));
    }
}

fn combo_setters(lines: &mut Vec<String>, var: &str, props: &ComboBoxProps) {
    items_setter(lines, var, &props.items);
    if let Some(selected) = &props.selected {
        lines.push(format!("ui_set_selected({var}, {})", quoted(selected)));
    }
    if props.editable {
        lines.push(format!("ui_set_editable({var}, true)"));
    }
}

fn list_setters(lines: &mut Vec<String>, var: &str, props: &ListProps) {
    if !props.columns.is_empty() {
        let titles: Vec<String> = props.columns.iter().map(|c| c.title.clone()).collect();
        lines.push(format!("ui_set_columns({var}, {})", string_array(&titles)));
    }
    items_setter(lines, var, &props.items);
}

fn tree_setters(lines: &mut Vec<String>, var: &str, props: &TreeViewProps) {
    let labels = tree_labels(&props.nodes);
    items_setter(lines, var, &labels);
}

fn tabs_setters(lines: &mut Vec<String>, var: &str, props: &TabsProps) {
    items_setter(lines, var, &props.tabs);
    lines.push(format!(
        "ui_set_selected_index({var}, {})",
        number(props.selected as f64)
    ));
}

fn menu_setters(lines: &mut Vec<String>, var: &str, props: &MenuProps) {
    let labels: Vec<String> = props
        .items
        .iter()
        .filter(|item| !item.separator)
        .map(|item| item.label.clone())
        .collect();
    items_setter(lines, var, &labels);
}

fn status_bar_setters(lines: &mut Vec<String>, var: &str, props: &StatusBarProps) {
    items_setter(lines, var, &props.parts);
}

fn toolbar_setters(lines: &mut Vec<String>, var: &str, props: &ToolbarProps) {
    let labels: Vec<String> = props
        .buttons
        .iter()
        .filter(|button| !button.separator)
        .map(|button| button.text.clone())
        .collect();
    items_setter(lines, var, &labels);
}

fn range_setters(lines: &mut Vec<String>, var: &str, props: &RangeProps) {
    lines.push(format!(
        "ui_set_range({var}, {}, {})",
        number(props.min),
        number(props.max)
    ));
    lines.push(format!("ui_set_value({var}, {})", number(props.value)));
}

fn slider_setters(lines: &mut Vec<String>, var: &str, props: &SliderProps) {
    range_setters(
        lines,
        var,
        &RangeProps {
            min: props.min,
            max: props.max,
            value: props.value,
        },
    );
    lines.push(format!(
        "ui_set_orientation({var}, {})",
        quoted(orientation(props.orientation))
    ));
}

/// Emits `ui_set_items` for a non-empty list; an empty list is left absent
/// because the constructor already starts empty.
fn items_setter(lines: &mut Vec<String>, var: &str, items: &[String]) {
    if !items.is_empty() {
        lines.push(format!("ui_set_items({var}, {})", string_array(items)));
    }
}

fn tree_labels(nodes: &[TreeNode]) -> Vec<String> {
    let mut labels = Vec::new();
    for node in nodes {
        labels.push(node.label.clone());
        labels.extend(tree_labels(&node.children));
    }
    labels
}

fn string_array(items: &[String]) -> String {
    let quoted_items: Vec<String> = items.iter().map(|item| quoted(item)).collect();
    format!("[{}]", quoted_items.join(", "))
}

fn anchor_name(anchor: Anchor) -> &'static str {
    match anchor {
        Anchor::TopLeft => "top_left",
        Anchor::Top => "top",
        Anchor::TopRight => "top_right",
        Anchor::Left => "left",
        Anchor::Center => "center",
        Anchor::Right => "right",
        Anchor::BottomLeft => "bottom_left",
        Anchor::Bottom => "bottom",
        Anchor::BottomRight => "bottom_right",
        Anchor::StretchHorizontal => "stretch_horizontal",
        Anchor::StretchVertical => "stretch_vertical",
        Anchor::Fill => "fill",
    }
}

fn orientation(value: Orientation) -> &'static str {
    match value {
        Orientation::Horizontal => "horizontal",
        Orientation::Vertical => "vertical",
    }
}

fn scroll_bars(value: ScrollBars) -> &'static str {
    match value {
        ScrollBars::None => "none",
        ScrollBars::Vertical => "vertical",
        ScrollBars::Horizontal => "horizontal",
        ScrollBars::Both => "both",
    }
}

/// The events wired for a kind, in a fixed order.
fn events(kind: &ControlKind) -> &'static [&'static str] {
    match kind {
        ControlKind::Button
        | ControlKind::Label
        | ControlKind::GroupBox
        | ControlKind::Panel(_)
        | ControlKind::FlowText(_)
        | ControlKind::Menu(_)
        | ControlKind::Toolbar(_) => &["click"],
        ControlKind::Edit(_)
        | ControlKind::CheckBox(_)
        | ControlKind::Slider(_)
        | ControlKind::ProgressBar(_)
        | ControlKind::ColorPicker(_) => &["change"],
        ControlKind::RadioGroup(_)
        | ControlKind::ComboBox(_)
        | ControlKind::ListView(_)
        | ControlKind::TreeView(_)
        | ControlKind::GridView(_)
        | ControlKind::Tabs(_) => &["select"],
        ControlKind::ScrollView(_) => &["scroll"],
        ControlKind::StatusBar(_) => &[],
    }
}

/// Formats a finite model number as a Dyon literal.
pub(super) fn number(value: f64) -> String {
    format!("{value}")
}

/// Turns a control name into a Dyon-identifier-safe suffix.
///
/// Validation allows only ASCII alphanumerics, `-` and `_`; any other character
/// (a hand-made model) collapses to `_`. A leading digit gets a `_` prefix so
/// the result is a legal identifier.
fn identifier(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    if out.is_empty() {
        out.push_str("Control");
    }
    if out.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        out.insert(0, '_');
    }
    out
}

/// Allocates handler names in emission order, disambiguating collisions that
/// sanitisation could introduce (for example `a-b` and `a_b`).
#[derive(Default)]
pub(super) struct Handlers {
    names: Vec<String>,
    used: HashSet<String>,
}

impl Handlers {
    fn register(&mut self, event: &str, control_name: &str, index: usize) -> String {
        let base = format!("on_{event}_{}", identifier(control_name));
        let mut candidate = base.clone();
        let mut suffix = index;
        while !self.used.insert(candidate.clone()) {
            suffix += 1;
            candidate = format!("{base}_{suffix}");
        }
        self.names.push(candidate.clone());
        candidate
    }

    /// The handler names, in the order they were first referenced.
    pub(super) fn names(&self) -> &[String] {
        &self.names
    }
}
