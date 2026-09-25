//! Validation rules for a parsed [`Form`].
//!
//! Kept separate from the data shape so the rules can be read and tested on
//! their own; every failure is a typed [`ValidationError`], never a panic.

use std::collections::HashSet;

use crate::error::ValidationError;
use crate::model::{Control, ControlKind, Form, RangeProps};

pub(crate) fn validate(form: &Form) -> Result<(), ValidationError> {
    check_form_name(form)?;
    check_form_size(form)?;

    let mut seen: HashSet<&str> = HashSet::new();
    for (index, control) in form.controls.iter().enumerate() {
        check_control_name(index, control)?;
        if !seen.insert(control.name.as_str()) {
            return Err(ValidationError::DuplicateControlName {
                name: control.name.clone(),
            });
        }
        check_bounds(form, control)?;
        check_kind(control)?;
    }
    Ok(())
}

fn check_form_name(form: &Form) -> Result<(), ValidationError> {
    if form.name.trim().is_empty() {
        return Err(ValidationError::EmptyFormName);
    }
    if !is_valid_name(&form.name) {
        return Err(ValidationError::InvalidFormName {
            name: form.name.clone(),
        });
    }
    Ok(())
}

fn check_form_size(form: &Form) -> Result<(), ValidationError> {
    if !form.size.width.is_positive() || !form.size.height.is_positive() {
        return Err(ValidationError::InvalidFormSize {
            width: form.size.width.get(),
            height: form.size.height.get(),
        });
    }
    Ok(())
}

fn check_control_name(index: usize, control: &Control) -> Result<(), ValidationError> {
    if control.name.trim().is_empty() {
        return Err(ValidationError::EmptyControlName { index });
    }
    if !is_valid_name(&control.name) {
        return Err(ValidationError::InvalidControlName {
            name: control.name.clone(),
        });
    }
    Ok(())
}

fn check_bounds(form: &Form, control: &Control) -> Result<(), ValidationError> {
    let (x, y, width, height) = (
        control.bounds.x.get(),
        control.bounds.y.get(),
        control.bounds.width.get(),
        control.bounds.height.get(),
    );
    let kind = control.kind.tag();

    if !control.bounds.width.is_positive() || !control.bounds.height.is_positive() {
        return Err(ValidationError::InvalidControlSize {
            name: control.name.clone(),
            kind,
        });
    }

    let form_width = form.size.width.get();
    let form_height = form.size.height.get();
    let inside = control.bounds.x.is_non_negative()
        && control.bounds.y.is_non_negative()
        && x + width <= form_width
        && y + height <= form_height;
    if !inside {
        return Err(ValidationError::ControlOutOfBounds {
            name: control.name.clone(),
            kind,
            x,
            y,
            width,
            height,
            form_width,
            form_height,
        });
    }
    Ok(())
}

fn check_kind(control: &Control) -> Result<(), ValidationError> {
    match &control.kind {
        ControlKind::RadioGroup(props) => {
            if props.items.is_empty() {
                return Err(no_items(control));
            }
            check_selection(control, &props.items, props.selected.as_deref())
        }
        ControlKind::ComboBox(props) => {
            check_selection(control, &props.items, props.selected.as_deref())
        }
        ControlKind::Tabs(props) => {
            if props.tabs.is_empty() {
                return Err(no_items(control));
            }
            if props.selected as usize >= props.tabs.len() {
                return Err(ValidationError::SelectionOutOfRange {
                    name: control.name.clone(),
                    kind: control.kind.tag(),
                    selected: props.selected,
                    len: props.tabs.len(),
                });
            }
            Ok(())
        }
        ControlKind::ProgressBar(props) => check_range(control, props),
        ControlKind::Slider(props) => check_range(
            control,
            &RangeProps {
                min: props.min,
                max: props.max,
                value: props.value,
            },
        ),
        ControlKind::ColorPicker(props) => {
            if let Some(color) = &props.color
                && !is_hex_color(color)
            {
                return Err(ValidationError::InvalidColor {
                    name: control.name.clone(),
                    value: color.clone(),
                });
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

fn no_items(control: &Control) -> ValidationError {
    ValidationError::NoItems {
        name: control.name.clone(),
        kind: control.kind.tag(),
    }
}

/// A present selection must name one of the control's items.
fn check_selection(
    control: &Control,
    items: &[String],
    selected: Option<&str>,
) -> Result<(), ValidationError> {
    let Some(value) = selected else {
        return Ok(());
    };
    if items.iter().any(|item| item == value) {
        return Ok(());
    }
    Err(ValidationError::UnknownSelection {
        name: control.name.clone(),
        kind: control.kind.tag(),
        value: value.to_owned(),
    })
}

fn check_range(control: &Control, props: &RangeProps) -> Result<(), ValidationError> {
    let finite = props.min.is_finite() && props.max.is_finite() && props.value.is_finite();
    if !finite || props.min >= props.max || props.value < props.min || props.value > props.max {
        return Err(ValidationError::InvalidRange {
            name: control.name.clone(),
            kind: control.kind.tag(),
            min: props.min,
            max: props.max,
            value: props.value,
        });
    }
    Ok(())
}

/// Names double as generated symbols, so keep them to a portable identifier set.
fn is_valid_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// `#` followed by exactly six hex digits, the format the color picker emits.
fn is_hex_color(value: &str) -> bool {
    let Some(digits) = value.strip_prefix('#') else {
        return false;
    };
    digits.len() == 6 && digits.chars().all(|c| c.is_ascii_hexdigit())
}
