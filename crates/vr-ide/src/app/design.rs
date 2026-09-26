//! The design-mode half of [`IdeApp`]: entering and leaving the mode, the
//! palette, the property writes and the form/pane anchoring.
//!
//! Kept beside `mod.rs` so neither file grows past the house limit; the split
//! is by responsibility rather than by visibility, so these methods still reach
//! the app's private fields.

use vr_forms::{Anchor, Dip, Form, Size};
use xui::prelude::*;

use crate::Msg;
use crate::designer;
use crate::msg::{PaletteKind, PropertyField};

use super::{DEMO_ENV, IdeApp, env_flag};

impl IdeApp {
    /// Enters or leaves design mode so the central pane matches
    /// [`IdeState::design`](crate::IdeState::design).
    ///
    /// Design mode is a mode, not a tab: the editor and the designer share the
    /// central slot, and only the visible one takes space. The inspector shows
    /// with the designer and is blanked when no control is selected.
    pub(crate) fn sync_design(&mut self, ui: &Ui<Msg>) {
        if self.state.design {
            if let Some(pane) = &self.design {
                pane.set_visible(true);
            }
            if let Some(pane) = &self.inspector {
                pane.set_visible(true);
            }
            self.refresh_inspector();
            let name = self.design.as_ref().map(|design| design.form().name);
            self.state.push_output(&format!(
                "Design mode on: {}",
                name.unwrap_or_else(|| "(no form)".to_owned())
            ));
            if env_flag(DEMO_ENV) {
                self.prepare_demo();
            }
        } else {
            if let Some(pane) = &self.design {
                pane.set_visible(false);
            }
            if let Some(pane) = &self.inspector {
                pane.set_visible(false);
            }
            self.state.push_output("Design mode off");
        }
        if let Some(host) = &self.editor {
            host.set_visible(!self.state.design);
        }
        ui.set_menu_checked(crate::menus::DESIGN, self.state.design);
        self.refresh_output();
        if let Some(bar) = &self.status {
            bar.set_text(0, &self.state.status_line());
        }
        ui.relayout();
    }

    /// Replaces the designer's form, e.g. when the explorer selects a `.vrform`.
    pub(crate) fn load_form_into_designer(&mut self, form: Form) {
        if let Some(design) = &mut self.design
            && let Err(error) = design.try_set_form(form)
        {
            self.state
                .push_output(&format!("Form rejected ({error}); keeping the current one"));
        }
        self.refresh_inspector();
    }

    /// Appends a palette control to the form and selects it.
    pub(crate) fn add_control(&mut self, kind: PaletteKind) {
        let Some(design) = &mut self.design else {
            self.state.set_status("No designer to add to");
            return;
        };
        design.add_control(designer::control_kind(kind));
        self.state.set_status(&format!("Added {kind:?}"));
        self.refresh_inspector();
    }

    /// Writes the inspector field the user committed back to the model.
    pub(crate) fn commit_property(&mut self, field: PropertyField) {
        let Some(value) = self
            .inspector
            .as_ref()
            .and_then(|inspector| inspector.read(field))
        else {
            // A numeric field that no longer parses: put the model value back.
            self.state
                .set_status(&format!("{}: expected a number", field.name()));
            self.refresh_inspector();
            return;
        };
        let applied = self
            .design
            .as_mut()
            .is_some_and(|design| design.set_property(field.name(), value));
        self.state.set_status(&if applied {
            format!("Set {}", field.name())
        } else {
            format!("{}: edit rejected", field.name())
        });
        self.refresh_inspector();
    }

    /// Re-anchors the selected control.
    pub(crate) fn set_anchor(&mut self, anchor: Anchor) {
        let applied = self
            .design
            .as_mut()
            .is_some_and(|design| design.set_anchor(anchor));
        if applied {
            self.state.set_status(&format!("Anchor: {anchor:?}"));
        }
        self.refresh_inspector();
    }

    /// Resizes the form to the designer pane's current size, so stretch and
    /// fill anchors preview what `apply_anchors` will do at run time.
    pub(crate) fn resize_form_to_pane(&mut self, ui: &Ui<Msg>) {
        let dpi = ui.dpi();
        let scale = if dpi == 0 { 1.0 } else { f64::from(dpi) / 96.0 };
        let size = self.design.as_ref().map(|design| {
            let bounds = design.bounds();
            Size {
                width: Dip::new(f64::from(bounds.width()) / scale),
                height: Dip::new(f64::from(bounds.height()) / scale),
            }
        });
        let Some(size) = size else {
            return;
        };
        if let Some(design) = &mut self.design {
            design.resize_form(size);
        }
        self.state.set_status(&format!(
            "Form resized to {} x {}",
            size.width.get(),
            size.height.get()
        ));
        self.refresh_inspector();
    }

    /// Rewrites the inspector from the selected control, blanking it when the
    /// selection is empty.
    pub(crate) fn refresh_inspector(&self) {
        let Some(inspector) = &self.inspector else {
            return;
        };
        let control = self.design.as_ref().and_then(|design| {
            let index = design.selected()?;
            let form = design.form();
            form.controls.as_slice().get(index).cloned()
        });
        inspector.show(control.as_ref());
    }

    /// Writes the designer's form back to the file it was loaded from.
    pub(crate) fn save_form(&mut self) {
        let Some(path) = self.form_path.clone() else {
            self.state
                .set_status("The form has no file path; it is the built-in sample");
            return;
        };
        let Some(design) = &self.design else {
            self.state.set_status("No designer to save");
            return;
        };
        match design.form().save(&path) {
            Ok(()) => {
                self.state.set_status(&format!("Saved {}", path.display()));
                self.state
                    .push_output(&format!("Saved form {}", path.display()));
            }
            Err(error) => self.state.set_status(&format!("Save failed: {error}")),
        }
        self.refresh_output();
    }

    /// The capture test's setup: select a control and make it a `Fill` anchor so
    /// a form resize visibly stretches it.
    fn prepare_demo(&mut self) {
        let Some(design) = &mut self.design else {
            return;
        };
        let mut form = design.form();
        let index = form
            .controls
            .iter()
            .position(|control| control.kind.tag() == "button")
            .unwrap_or(0);
        if let Some(control) = form.controls.get_mut(index) {
            control.anchor = Anchor::Fill;
        }
        if design.try_set_form(form).is_ok() {
            design.select(Some(index));
        }
        self.refresh_inspector();
    }
}
