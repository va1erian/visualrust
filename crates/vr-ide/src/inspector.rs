//! The design-mode property inspector.
//!
//! A [`Panel`] of native fields over one strip of the shell: a field per model
//! property, a checkbox per boolean, and an anchor list. The panel is a plain
//! [`AsControl`], so the main layout places it like any other widget; its own
//! children are positioned by a nested layout the panel owns.
//!
//! Edits commit on Enter or on losing focus rather than on every keystroke: the
//! shell writes the model and then rewrites the fields from it, and a live
//! `on_change` would re-enter that refresh as a fresh edit. Committing on submit
//! keeps the refresh path free of feedback.

use vr_forms::design::PropertyValue;
use vr_forms::{Anchor, Control as ModelControl};
use xui::prelude::*;

use crate::Msg;
use crate::msg::PropertyField;

/// Every anchor scheme the selector offers, with the label shown for it.
const ANCHORS: [(&str, Anchor); 12] = [
    ("Top-left", Anchor::TopLeft),
    ("Top", Anchor::Top),
    ("Top-right", Anchor::TopRight),
    ("Left", Anchor::Left),
    ("Centre", Anchor::Center),
    ("Right", Anchor::Right),
    ("Bottom-left", Anchor::BottomLeft),
    ("Bottom", Anchor::Bottom),
    ("Bottom-right", Anchor::BottomRight),
    ("Stretch H", Anchor::StretchHorizontal),
    ("Stretch V", Anchor::StretchVertical),
    ("Fill", Anchor::Fill),
];

/// The field captions above each edit, in layout order.
const LABELS: [&str; 7] = ["Name", "Text", "X", "Y", "Width", "Height", "Anchor"];

/// The right-hand-side inspector: one row per editable property.
pub struct PropertyInspector {
    panel: Panel,
    /// Kept only so the caption windows outlive the layout handles the panel
    /// holds; the shell never reads a caption back.
    #[allow(dead_code)]
    labels: Vec<Label>,
    name: Edit<Msg>,
    text: Edit<Msg>,
    x: Edit<Msg>,
    y: Edit<Msg>,
    width: Edit<Msg>,
    height: Edit<Msg>,
    enabled: CheckBox<Msg>,
    visible: CheckBox<Msg>,
    anchor: ComboBox<Anchor, Msg>,
}

impl PropertyInspector {
    /// Builds the inspector as a child of the window behind `ui`.
    pub fn new(ui: &mut Ui<Msg>) -> xui::Result<PropertyInspector> {
        let panel = Panel::new(ui)?;
        let mut pui = panel.ui(ui);

        let mut labels = Vec::with_capacity(LABELS.len());
        for caption in LABELS {
            labels.push(Label::new(&mut pui, caption_bounds(), caption)?);
        }

        let name = text_field(&mut pui, PropertyField::Name)?;
        let text = text_field(&mut pui, PropertyField::Text)?;
        let x = text_field(&mut pui, PropertyField::X)?;
        let y = text_field(&mut pui, PropertyField::Y)?;
        let width = text_field(&mut pui, PropertyField::Width)?;
        let height = text_field(&mut pui, PropertyField::Height)?;
        let enabled = CheckBox::new(&mut pui, "Enabled")?
            .on_toggle(|_| Some(Msg::Commit(PropertyField::Enabled)));
        let visible = CheckBox::new(&mut pui, "Visible")?
            .on_toggle(|_| Some(Msg::Commit(PropertyField::Visible)));
        let anchor =
            ComboBox::new(&mut pui, ANCHORS)?.on_select(|anchor| Some(Msg::SetAnchor(*anchor)));

        let layout = Layout::column()
            .spacing(dip(3.0))
            .margins(Insets::all(dip(8.0)))
            .item(&labels[0])
            .item(&name)
            .item(&labels[1])
            .item(&text)
            .item(&labels[2])
            .item(&x)
            .item(&labels[3])
            .item(&y)
            .item(&labels[4])
            .item(&width)
            .item(&labels[5])
            .item(&height)
            .item(&enabled)
            .item(&visible)
            .item(&labels[6])
            .item(&anchor);
        panel.set_layout(layout);

        let inspector = PropertyInspector {
            panel,
            labels,
            name,
            text,
            x,
            y,
            width,
            height,
            enabled,
            visible,
            anchor,
        };
        inspector.show(None);
        Ok(inspector)
    }

    /// Rewrites every field from `control`, or blanks and disables them all when
    /// no control is selected.
    pub fn show(&self, control: Option<&ModelControl>) {
        match control {
            Some(control) => {
                self.name.set_text(&control.name);
                self.text.set_text(&control.text);
                self.x.set_text(&number_text(control.bounds.x.get()));
                self.y.set_text(&number_text(control.bounds.y.get()));
                self.width
                    .set_text(&number_text(control.bounds.width.get()));
                self.height
                    .set_text(&number_text(control.bounds.height.get()));
                self.enabled.set_checked(control.enabled);
                self.visible.set_checked(control.visible);
                self.anchor.set_selected(&control.anchor);
                self.set_fields_enabled(true);
            }
            None => {
                for edit in self.edits() {
                    edit.set_text("");
                }
                self.enabled.set_checked(false);
                self.visible.set_checked(false);
                self.set_fields_enabled(false);
            }
        }
    }

    /// The value currently shown for `field`, or `None` when a numeric field
    /// does not parse (so the model is left untouched).
    pub fn read(&self, field: PropertyField) -> Option<PropertyValue> {
        match field {
            PropertyField::Name => Some(PropertyValue::Text(self.name.text())),
            PropertyField::Text => Some(PropertyValue::Text(self.text.text())),
            PropertyField::X => number(&self.x),
            PropertyField::Y => number(&self.y),
            PropertyField::Width => number(&self.width),
            PropertyField::Height => number(&self.height),
            PropertyField::Enabled => Some(PropertyValue::Bool(self.enabled.is_checked())),
            PropertyField::Visible => Some(PropertyValue::Bool(self.visible.is_checked())),
        }
    }

    /// Greys or ungreys every field with the selection.
    fn set_fields_enabled(&self, enabled: bool) {
        for edit in self.edits() {
            edit.set_enabled(enabled);
        }
        self.enabled.set_enabled(enabled);
        self.visible.set_enabled(enabled);
        self.anchor.set_enabled(enabled);
        for label in &self.labels {
            label.set_enabled(enabled);
        }
    }

    /// Every editable field, so enabling and clearing stay in step.
    fn edits(&self) -> [&Edit<Msg>; 6] {
        [
            &self.name,
            &self.text,
            &self.x,
            &self.y,
            &self.width,
            &self.height,
        ]
    }
}

impl AsControl for PropertyInspector {
    fn control(&self) -> &Control {
        self.panel.control()
    }
}

/// A single-line field that commits its property on Enter or focus loss.
fn text_field(ui: &mut Ui<Msg>, field: PropertyField) -> xui::Result<Edit<Msg>> {
    Ok(Edit::single_line(ui)?
        .on_submit(move || Some(Msg::Commit(field)))
        .on_focus(move |focused| (!focused).then_some(Msg::Commit(field))))
}

/// A numeric field parsed out of its text.
fn number(edit: &Edit<Msg>) -> Option<PropertyValue> {
    edit.text()
        .trim()
        .parse::<f64>()
        .ok()
        .map(PropertyValue::Number)
}

/// `value` as the shortest text that parses back to it.
fn number_text(value: f64) -> String {
    format!("{value}")
}

/// A caption's natural bounds; the panel's layout owns the final position.
fn caption_bounds() -> Rect {
    Rect::new(0, 0, 200, 18)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_round_trip_through_their_text() {
        for value in [0.0, 24.0, 8.5, 300.0] {
            assert_eq!(number_text(value).parse::<f64>(), Ok(value));
        }
    }

    #[test]
    fn anchor_labels_cover_every_scheme_once() {
        let mut seen: Vec<Anchor> = Vec::new();
        for (_, anchor) in ANCHORS {
            assert!(!seen.contains(&anchor), "{anchor:?} listed twice");
            seen.push(anchor);
        }
        assert_eq!(seen.len(), 12);
    }
}
