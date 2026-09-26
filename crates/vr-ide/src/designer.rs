//! Design mode: the central pane hosting the real [`FormDesigner`].
//!
//! The designer from `vr-forms` is built on `xui`'s native widget layer, so it
//! can be dropped straight into this shell's `xui_win32` app as a layout item
//! (`FormDesigner: AsControl`). Design mode remains a *mode*, not a tab: the
//! editor and the designer share the central slot and only the visible one
//! occupies space.

use vr_forms::design::{DesignerError, DesignerEvent, FormDesigner, PropertyValue};
use vr_forms::model::{ChoiceProps, ComboBoxProps};
use vr_forms::{Anchor, ControlKind, Dip, Form, Size};
use xui::prelude::*;

use crate::Msg;
use crate::msg::PaletteKind;

/// The design grid step, in design units.
pub const GRID_DIP: f64 = 8.0;

/// The central form-designer pane.
///
/// It is a thin owner of the `vr-forms` [`FormDesigner`], keeping the shell's
/// message type in one place and giving the layout a stable `AsControl`.
pub struct DesignPane {
    designer: FormDesigner<Msg>,
}

impl DesignPane {
    /// Hosts `form` as the central designer.
    pub fn new(ui: &mut Ui<Msg>, form: Form) -> std::result::Result<DesignPane, DesignerError> {
        let designer = FormDesigner::new(ui, form, Dip::new(GRID_DIP))?;
        Ok(DesignPane { designer })
    }

    /// Maps designer events to the shell's messages.
    pub fn on_change(&mut self, mapper: impl Fn(DesignerEvent) -> Option<Msg> + 'static) {
        self.designer.on_change(mapper);
    }

    /// The current form model.
    pub fn form(&self) -> Form {
        self.designer.form()
    }

    /// Replaces the form; an invalid model is refused rather than put on screen.
    pub fn try_set_form(&mut self, form: Form) -> std::result::Result<(), DesignerError> {
        self.designer.try_set_form(form)
    }

    /// Selects a control (or clears the selection).
    pub fn select(&self, index: Option<usize>) {
        self.designer.select(index);
    }

    /// The selected control's index, if any.
    pub fn selected(&self) -> Option<usize> {
        self.designer.selected()
    }

    /// Appends a control of `kind`, selects it and returns its index.
    pub fn add_control(&mut self, kind: ControlKind) -> usize {
        self.designer.add_control(kind)
    }

    /// Writes a property of the selected control, returning whether it applied.
    pub fn set_property(&mut self, name: &str, value: PropertyValue) -> bool {
        self.designer.set_property(name, value)
    }

    /// Re-anchors the selected control, keeping the selection.
    ///
    /// The designer's property palette has no anchor key, so the anchor is
    /// changed by rebuilding the model and re-selecting. A failed validation
    /// leaves the model as it was.
    pub fn set_anchor(&mut self, anchor: Anchor) -> bool {
        let Some(index) = self.selected() else {
            return false;
        };
        let mut form = self.form();
        match form.controls.as_mut_slice().get_mut(index) {
            Some(control) => control.anchor = anchor,
            None => return false,
        }
        if self.try_set_form(form).is_err() {
            return false;
        }
        self.select(Some(index));
        true
    }

    /// Anchors every control for a form resize to `new_size`.
    pub fn resize_form(&mut self, new_size: Size) {
        self.designer.resize_form(new_size);
    }
}

impl AsControl for DesignPane {
    fn control(&self) -> &Control {
        self.designer.control()
    }
}

/// The concrete model kind a palette entry adds, with defaults for the settings
/// a fresh control needs but the palette does not ask about.
pub fn control_kind(kind: PaletteKind) -> ControlKind {
    match kind {
        PaletteKind::Button => ControlKind::Button,
        PaletteKind::Label => ControlKind::Label,
        PaletteKind::Edit => ControlKind::Edit(Default::default()),
        PaletteKind::CheckBox => ControlKind::CheckBox(Default::default()),
        PaletteKind::ComboBox => ControlKind::ComboBox(ComboBoxProps {
            items: vec!["Item 1".to_owned()],
            selected: None,
            editable: false,
        }),
        PaletteKind::ProgressBar => ControlKind::ProgressBar(Default::default()),
        PaletteKind::Slider => ControlKind::Slider(Default::default()),
        PaletteKind::GroupBox => ControlKind::GroupBox,
        // A radio group must own at least one option to be a valid model, so a
        // fresh one starts with two rather than an empty (unvalidatable) list.
        PaletteKind::RadioGroup => ControlKind::RadioGroup(ChoiceProps {
            items: vec!["Option 1".to_owned(), "Option 2".to_owned()],
            selected: None,
        }),
    }
}

/// Every palette entry, in menu order, so the Form menu and the tests agree.
pub const PALETTE: [PaletteKind; 9] = [
    PaletteKind::Button,
    PaletteKind::Label,
    PaletteKind::Edit,
    PaletteKind::CheckBox,
    PaletteKind::ComboBox,
    PaletteKind::ProgressBar,
    PaletteKind::Slider,
    PaletteKind::GroupBox,
    PaletteKind::RadioGroup,
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_palette_entry_maps_to_its_model_tag() {
        let cases = [
            (PaletteKind::Button, "button"),
            (PaletteKind::Label, "label"),
            (PaletteKind::Edit, "edit"),
            (PaletteKind::CheckBox, "check_box"),
            (PaletteKind::ComboBox, "combo_box"),
            (PaletteKind::ProgressBar, "progress_bar"),
            (PaletteKind::Slider, "slider"),
            (PaletteKind::GroupBox, "group_box"),
            (PaletteKind::RadioGroup, "radio_group"),
        ];
        for (palette, tag) in cases {
            assert_eq!(control_kind(palette).tag(), tag);
        }
    }

    #[test]
    fn a_palette_control_validates_inside_the_sample_form() {
        let mut form = crate::explorer::sample_form();
        for (index, palette) in PALETTE.iter().enumerate() {
            // Add at a free strip so the newly appended control stays inside
            // the form and the whole model still validates.
            let kind = control_kind(*palette);
            form.controls.push(vr_forms::Control {
                kind,
                name: format!("Added{index}"),
                bounds: vr_forms::Bounds {
                    x: Dip::new(8.0),
                    y: Dip::new(8.0),
                    width: Dip::new(40.0),
                    height: Dip::new(16.0),
                },
                text: String::new(),
                enabled: true,
                visible: true,
                tooltip: None,
                anchor: Anchor::TopLeft,
            });
        }
        form.validate().expect("every palette kind validates");
    }
}
