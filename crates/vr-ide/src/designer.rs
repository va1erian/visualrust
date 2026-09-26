//! Design mode: the central form pane.
//!
//! ## Why this is a preview, not the live `DesignerSurface`
//!
//! `vr_forms::designer::DesignerSurface` is built against `xui_core::Ui` (the
//! portable core app), while the IDE shell uses `xui_win32::Ui` (the native
//! control app). The two `Ui` types are distinct in the pinned xui rev, so the
//! surface cannot be constructed from the shell's window. Follow-up work is to
//! converge the apps (or expose a bridge) in xui; until then design mode renders
//! the selected [`Form`] model in a themed read-only pane.
//!
//! ## Why design mode is a mode anyway
//!
//! The designer could not be a `tabs!` page even once the types converge:
//! `DesignerSurface::new` calls `Ui::set_design_mode(true)` on the whole window,
//! which makes *every* widget ignore its own input, so the editor and the
//! designer are mutually exclusive. Leaving design mode restores the editor.

use vr_forms::Form;
use xui::prelude::*;

use crate::Msg;

/// A read-only pane that renders the form model on screen in design mode.
pub struct DesignPane {
    edit: Edit<Msg>,
}

impl DesignPane {
    /// Creates the pane as a child of the window behind `ui`.
    pub fn new(ui: &mut Ui<Msg>) -> xui::Result<DesignPane> {
        let edit = Edit::multi_line(ui)?.read_only(true);
        Ok(DesignPane { edit })
    }

    /// Replaces the pane's text.
    pub fn set_text(&self, text: &str) {
        self.edit.set_text(text);
    }
}

impl AsControl for DesignPane {
    fn control(&self) -> &Control {
        self.edit.control()
    }
}

/// Renders a form (or the empty selection) as the design pane's text.
pub fn render(form: Option<&Form>) -> String {
    let mut lines: Vec<String> = vec![
        "Form Designer".to_owned(),
        "The live xui DesignerSurface is not available in this shell: its \
         xui_core::Ui differs from the shell's xui_win32::Ui. This pane renders \
         the form model instead."
            .to_owned(),
        String::new(),
    ];
    match form {
        Some(form) => {
            lines.push(format!(
                "{}  ({} x {})",
                form.name,
                form.size.width.get(),
                form.size.height.get()
            ));
            lines.push(format!("{} control(s)", form.controls.len()));
            lines.push(String::new());
            for (index, control) in form.controls.iter().enumerate() {
                let bounds = &control.bounds;
                lines.push(format!(
                    "[{index}] {} `{}`  at ({}, {})  {} x {}{}",
                    control.kind.tag(),
                    control.name,
                    bounds.x.get(),
                    bounds.y.get(),
                    bounds.width.get(),
                    bounds.height.get(),
                    text_suffix(&control.text),
                ));
            }
        }
        None => lines.push("No form selected".to_owned()),
    }
    lines.join("\r\n")
}

/// Appends a quoted caption to a preview line when the control carries text.
fn text_suffix(text: &str) -> String {
    if text.is_empty() {
        String::new()
    } else {
        format!("  \"{text}\"")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_empty_selection_renders_a_placeholder() {
        let text = render(None);
        assert!(text.contains("No form selected"));
    }

    #[test]
    fn a_form_lists_its_name_and_controls() {
        let form = crate::explorer::sample_form();
        let text = render(Some(&form));
        assert!(text.contains("SampleForm"));
        assert!(text.contains("Greet"));
        assert!(text.contains("control(s)"));
    }

    #[test]
    fn the_form_size_renders_in_design_units() {
        let form = crate::explorer::sample_form();
        assert!(form.size.width.get() > 0.0);
        let text = render(Some(&form));
        assert!(text.contains(&form.size.width.get().to_string()));
    }
}
