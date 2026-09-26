//! The design-time property palette: reading and writing one property of a
//! control, and adding a control at a sensible default position.
//!
//! The property operations are free functions over the [`Form`] model so they
//! are unit-tested without a window. Every mutation is validated against the
//! whole form and rolled back when it would break a structural rule (an empty
//! name, a duplicate name, a control moved outside the form), so the model the
//! designer holds is always one [`Form::validate`] accepts.

use crate::model::{
    Anchor, Bounds, ColorPickerProps, Control, ControlKind, Dip, Form, RangeProps, SliderProps,
};

use super::interact::snap;

/// One property value in the design-time palette, mapped to the model fields
/// the property list exposes.
#[derive(Clone, Debug, PartialEq)]
pub enum PropertyValue {
    /// A string property (`name`, `text`).
    Text(String),
    /// A numeric property (`x`, `y`, `width`, `height`).
    Number(f64),
    /// A boolean property (`enabled`, `visible`).
    Bool(bool),
}

/// The distance from the form origin a first control is placed at, in design
/// units. Each added control flows down from it and wraps into a new column.
const PLACEMENT_ORIGIN_DIP: f64 = 16.0;

/// The vertical gap between two stacked default controls, in design units.
const PLACEMENT_GAP_DIP: f64 = 8.0;

/// The horizontal pitch between placement columns, in design units.
const PLACEMENT_COLUMN_DIP: f64 = 220.0;

/// Appends a control of `kind` at a default position and returns its index.
///
/// The kind is normalised first so an empty radio group or an out-of-range
/// slider, which callers routinely hand in from a palette entry, still produces
/// a form that validates. The name is unique within the form.
pub fn add(form: &mut Form, kind: ControlKind, grid: f64) -> usize {
    let kind = normalize(kind);
    let bounds = default_placement(form, &kind, grid);
    let name = unique_name(form, &kind);
    let control = Control {
        kind,
        name,
        bounds,
        text: String::new(),
        enabled: true,
        visible: true,
        tooltip: None,
        anchor: Anchor::TopLeft,
    };
    form.controls.push(control);
    form.controls.len() - 1
}

/// The current value of `name` on control `index`, or `None` when the index is
/// out of range or the name is not an editable property.
pub fn get(form: &Form, index: usize, name: &str) -> Option<PropertyValue> {
    let control = form.controls.get(index)?;
    Some(match name {
        "name" => PropertyValue::Text(control.name.clone()),
        "text" => PropertyValue::Text(control.text.clone()),
        "x" => PropertyValue::Number(control.bounds.x.get()),
        "y" => PropertyValue::Number(control.bounds.y.get()),
        "width" => PropertyValue::Number(control.bounds.width.get()),
        "height" => PropertyValue::Number(control.bounds.height.get()),
        "enabled" => PropertyValue::Bool(control.enabled),
        "visible" => PropertyValue::Bool(control.visible),
        _ => return None,
    })
}

/// Writes `value` to property `name` on control `index`, returning whether the
/// write applied.
///
/// A mismatched value type, an unknown property, or a change that would leave
/// the form invalid is rejected without touching the model.
pub fn set(form: &mut Form, index: usize, name: &str, value: PropertyValue) -> bool {
    let Some(control) = form.controls.get_mut(index) else {
        return false;
    };
    let previous = control.clone();
    let applied = match (name, value) {
        ("name", PropertyValue::Text(text)) => {
            control.name = text;
            true
        }
        ("text", PropertyValue::Text(text)) => {
            control.text = text;
            true
        }
        ("x", PropertyValue::Number(value)) => {
            control.bounds.x = Dip::new(value);
            true
        }
        ("y", PropertyValue::Number(value)) => {
            control.bounds.y = Dip::new(value);
            true
        }
        ("width", PropertyValue::Number(value)) => {
            control.bounds.width = Dip::new(value);
            true
        }
        ("height", PropertyValue::Number(value)) => {
            control.bounds.height = Dip::new(value);
            true
        }
        ("enabled", PropertyValue::Bool(value)) => {
            control.enabled = value;
            true
        }
        ("visible", PropertyValue::Bool(value)) => {
            control.visible = value;
            true
        }
        _ => false,
    };
    if !applied {
        return false;
    }
    if form.validate().is_err() {
        form.controls[index] = previous;
        return false;
    }
    true
}

/// A default position and size for a new control: it flows down from the form
/// origin and wraps into a new column when the column is full, snapped to
/// `grid` and clamped so it always fits inside the form.
fn default_placement(form: &Form, kind: &ControlKind, grid: f64) -> Bounds {
    let (default_width, default_height) = default_size(kind);
    let width = default_width.min(form.size.width.get());
    let height = default_height.min(form.size.height.get());
    let origin = snap(PLACEMENT_ORIGIN_DIP, grid);
    // Snap the pitch, not the position, so the gap survives a coarse grid: a
    // row then advances by a whole multiple of the grid and never overlaps the
    // control above it.
    let pitch = snap(height + PLACEMENT_GAP_DIP, grid).max(grid.max(0.0));
    let usable = (form.size.height.get() - origin).max(pitch);
    let per_column = (usable / pitch).floor().max(1.0) as usize;
    let index = form.controls.len();
    let column = index / per_column;
    let row = index % per_column;
    let max_x = (form.size.width.get() - width).max(0.0);
    let max_y = (form.size.height.get() - height).max(0.0);
    let x = (origin + column as f64 * PLACEMENT_COLUMN_DIP).clamp(0.0, max_x);
    let y = (origin + row as f64 * pitch).clamp(0.0, max_y);
    Bounds {
        x: Dip::new(x),
        y: Dip::new(y),
        width: Dip::new(width),
        height: Dip::new(height),
    }
}

/// The default design size of a widget of `kind`, chosen to look like the
/// widget rather than a uniform box.
fn default_size(kind: &ControlKind) -> (f64, f64) {
    match kind {
        ControlKind::Button => (88.0, 28.0),
        ControlKind::Label => (90.0, 20.0),
        ControlKind::Edit(_) => (160.0, 24.0),
        ControlKind::CheckBox(_) => (110.0, 24.0),
        ControlKind::RadioGroup(_) => (140.0, 72.0),
        ControlKind::ComboBox(_) => (150.0, 24.0),
        ControlKind::ListView(_) | ControlKind::GridView(_) => (200.0, 120.0),
        ControlKind::TreeView(_) => (180.0, 140.0),
        ControlKind::GroupBox | ControlKind::Panel(_) => (200.0, 120.0),
        ControlKind::ScrollView(_) => (200.0, 140.0),
        ControlKind::Tabs(_) => (260.0, 160.0),
        ControlKind::Menu(_) | ControlKind::Toolbar(_) | ControlKind::StatusBar(_) => (200.0, 28.0),
        ControlKind::ProgressBar(_) => (160.0, 16.0),
        ControlKind::Slider(_) => (160.0, 28.0),
        ControlKind::ColorPicker(_) => (80.0, 24.0),
        ControlKind::FlowText(_) => (200.0, 60.0),
    }
}

/// A name unique within `form` for a control of `kind`.
///
/// The prefix is the kind's PascalCase tag and the number is a running
/// sequence, so adding a button then a label names them `Button1` and `Label2`
/// as a designer palette conventionally does. The sequence keeps rising until
/// the name is free, which stays unique even if a form was loaded with names
/// that fit the pattern.
fn unique_name(form: &Form, kind: &ControlKind) -> String {
    let prefix = pascal_case(kind.tag());
    let mut number = form.controls.len() + 1;
    loop {
        let candidate = format!("{prefix}{number}");
        if !form.controls.iter().any(|c| c.name == candidate) {
            return candidate;
        }
        number += 1;
    }
}

/// `snake_case` to `PascalCase`: `radio_group` becomes `RadioGroup`.
fn pascal_case(tag: &str) -> String {
    let mut out = String::with_capacity(tag.len());
    let mut upper = true;
    for ch in tag.chars() {
        if ch == '_' {
            upper = true;
        } else if upper {
            out.extend(ch.to_uppercase());
            upper = false;
        } else {
            out.push(ch);
        }
    }
    out
}

/// Replaces kind-specific settings that would fail validation with usable
/// defaults, so a palette entry always yields a valid control.
fn normalize(kind: ControlKind) -> ControlKind {
    match kind {
        ControlKind::RadioGroup(mut props) => {
            if props.items.is_empty() {
                props.items = vec!["Option 1".to_owned(), "Option 2".to_owned()];
                props.selected = None;
            } else if let Some(selected) = &props.selected
                && !props.items.iter().any(|item| item == selected)
            {
                props.selected = None;
            }
            ControlKind::RadioGroup(props)
        }
        ControlKind::ComboBox(mut props) => {
            if let Some(selected) = &props.selected
                && !props.items.iter().any(|item| item == selected)
            {
                props.selected = None;
            }
            ControlKind::ComboBox(props)
        }
        ControlKind::Tabs(mut props) => {
            if props.tabs.is_empty() {
                props.tabs = vec!["Tab 1".to_owned()];
                props.selected = 0;
            } else if props.selected as usize >= props.tabs.len() {
                props.selected = 0;
            }
            ControlKind::Tabs(props)
        }
        ControlKind::ProgressBar(props) => {
            if valid_range(props.min, props.max, props.value) {
                ControlKind::ProgressBar(props)
            } else {
                ControlKind::ProgressBar(default_range())
            }
        }
        ControlKind::Slider(props) => {
            if valid_range(props.min, props.max, props.value) {
                ControlKind::Slider(props)
            } else {
                ControlKind::Slider(SliderProps {
                    min: 0.0,
                    max: 100.0,
                    value: 0.0,
                    ..props
                })
            }
        }
        ControlKind::ColorPicker(props) => {
            let color = match &props.color {
                Some(value) if !is_hex_color(value) => None,
                other => other.clone(),
            };
            ControlKind::ColorPicker(ColorPickerProps { color })
        }
        other => other,
    }
}

fn default_range() -> RangeProps {
    RangeProps {
        min: 0.0,
        max: 100.0,
        value: 0.0,
    }
}

fn valid_range(min: f64, max: f64, value: f64) -> bool {
    min.is_finite()
        && max.is_finite()
        && value.is_finite()
        && min < max
        && value >= min
        && value <= max
}

fn is_hex_color(value: &str) -> bool {
    let Some(digits) = value.strip_prefix('#') else {
        return false;
    };
    digits.len() == 6 && digits.chars().all(|c| c.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ChoiceProps, EditProps, Size};

    fn form() -> Form {
        Form {
            name: "Main".to_owned(),
            size: Size {
                width: Dip::new(400.0),
                height: Dip::new(300.0),
            },
            controls: Vec::new(),
        }
    }

    #[test]
    fn add_places_a_unique_named_control_and_returns_its_index() {
        let mut form = form();
        let button = add(&mut form, ControlKind::Button, 8.0);
        let label = add(&mut form, ControlKind::Label, 8.0);

        assert_eq!(button, 0);
        assert_eq!(label, 1);
        assert_eq!(form.controls[0].name, "Button1");
        assert_eq!(form.controls[1].name, "Label2");
        form.validate().expect("added controls form a valid form");
    }

    #[test]
    fn add_avoids_a_name_already_in_the_form() {
        let mut form = form();
        form.controls.push(Control {
            kind: ControlKind::Button,
            name: "Button1".to_owned(),
            bounds: Bounds {
                x: Dip::new(0.0),
                y: Dip::new(0.0),
                width: Dip::new(10.0),
                height: Dip::new(10.0),
            },
            text: String::new(),
            enabled: true,
            visible: true,
            tooltip: None,
            anchor: Anchor::TopLeft,
        });
        add(&mut form, ControlKind::Button, 8.0);
        assert_eq!(form.controls[1].name, "Button2");
    }

    #[test]
    fn add_normalises_a_kind_that_would_not_validate() {
        let mut form = form();
        add(
            &mut form,
            ControlKind::RadioGroup(ChoiceProps::default()),
            8.0,
        );
        add(&mut form, ControlKind::Edit(EditProps::default()), 8.0);
        form.validate().expect("normalised kinds validate");
    }

    #[test]
    fn add_places_inside_the_form_and_snapped() {
        let mut form = form();
        // The second control cascades one grid step and lands on the grid.
        add(&mut form, ControlKind::Button, 8.0);
        add(&mut form, ControlKind::Button, 8.0);
        let bounds = form.controls[1].bounds;
        assert_eq!(bounds.x.get() % 8.0, 0.0);
        assert_eq!(bounds.y.get() % 8.0, 0.0);
        form.validate().expect("placed inside the form");
    }

    #[test]
    fn properties_round_trip_through_get_and_set() {
        let mut form = form();
        add(&mut form, ControlKind::Button, 8.0);

        assert!(set(&mut form, 0, "text", PropertyValue::Text("OK".into())));
        assert_eq!(
            get(&form, 0, "text"),
            Some(PropertyValue::Text("OK".into()))
        );

        assert!(set(&mut form, 0, "x", PropertyValue::Number(40.0)));
        assert_eq!(get(&form, 0, "x"), Some(PropertyValue::Number(40.0)));

        assert!(set(&mut form, 0, "visible", PropertyValue::Bool(false)));
        assert_eq!(get(&form, 0, "visible"), Some(PropertyValue::Bool(false)));

        assert!(set(&mut form, 0, "name", PropertyValue::Text("Go".into())));
        assert_eq!(
            get(&form, 0, "name"),
            Some(PropertyValue::Text("Go".into()))
        );
    }

    #[test]
    fn set_rejects_a_mismatched_type_and_an_unknown_property() {
        let mut form = form();
        add(&mut form, ControlKind::Button, 8.0);
        assert!(!set(&mut form, 0, "x", PropertyValue::Text("nope".into())));
        assert!(!set(&mut form, 0, "colour", PropertyValue::Bool(true)));
        assert_eq!(
            get(&form, 0, "colour"),
            None,
            "unknown properties are not exposed"
        );
    }

    #[test]
    fn set_rolls_back_a_change_that_breaks_validation() {
        let mut form = form();
        add(&mut form, ControlKind::Button, 8.0);
        let before = form.controls[0].bounds;
        // Negative coordinates are out of bounds, so the write is rejected.
        assert!(!set(&mut form, 0, "x", PropertyValue::Number(-10.0)));
        assert_eq!(form.controls[0].bounds, before);
        // An empty (or invalid) name is rejected too.
        assert!(!set(
            &mut form,
            0,
            "name",
            PropertyValue::Text(String::new())
        ));
        assert_eq!(form.controls[0].name, "Button1");
    }
}
