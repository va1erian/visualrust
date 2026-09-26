//! The IDE view model.
//!
//! The shell is a small unidirectional loop:
//!
//! ```text
//!   chrome (menus / toolbar / accelerators / timer)
//!         |  Msg
//!         v
//!   IdeState::apply     pure reducer, no xui types
//!         |  Effect { quit, theme_changed, layout_changed, status_changed,
//!         |            design_changed, output_changed }
//!         v
//!   IdeApp::update      the only code that touches the live window
//! ```
//!
//! Widgets never mutate state directly. They enqueue a [`Msg`], the reducer
//! folds it into [`IdeState`] and returns the [`Effect`] describing what the
//! window must do about it. Because the reducer is pure, the shell's behaviour
//! can be tested without creating a window, and because xui delivers one
//! queued message at a time, `update` is never re-entered.
//!
//! [`Msg`]: crate::Msg

use crate::Msg;
use crate::explorer::{ExplorerItem, ItemKind};

/// What the window must re-apply after the reducer ran.
///
/// The flags are a diff, not a command: `update` reads the ones that are set
/// and leaves the rest of the window untouched, so a message that only changes
/// a status string does not repaint the menus.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Effect {
    /// Leave the message loop.
    pub quit: bool,
    /// Switch the window (and its controls and menus) to `dark`.
    pub theme_changed: bool,
    /// Re-run the layout because a bar was shown or hidden.
    pub layout_changed: bool,
    /// Refresh the status bar text.
    pub status_changed: bool,
    /// Enter or leave design mode; the central pane must be rebuilt.
    pub design_changed: bool,
    /// Refresh the output pane text.
    pub output_changed: bool,
}

/// The shell's state, independent of any window.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IdeState {
    /// Whether the dark palette is selected.
    pub dark: bool,
    /// Whether the toolbar is part of the layout.
    pub toolbar_visible: bool,
    /// Whether the status bar is part of the layout.
    pub status_visible: bool,
    /// The text shown in the status bar's first part.
    pub status: String,
    /// The name of the open document, when the editor has one.
    pub document: Option<String>,
    /// Whether the editor's text differs from the file on disk.
    pub dirty: bool,
    /// Whether the (future) project runner is active.
    pub running: bool,
    /// The loaded project's name, when one was resolved.
    pub project: Option<String>,
    /// Every explorer row, indexed the same way the `TreeView` keys them.
    pub items: Vec<ExplorerItem>,
    /// The selected explorer row.
    pub selected: Option<usize>,
    /// Whether the central pane shows the form designer instead of the editor.
    pub design: bool,
    /// The output pane's log, oldest first.
    pub output: Vec<String>,
}

impl Default for IdeState {
    fn default() -> IdeState {
        IdeState {
            dark: true,
            toolbar_visible: true,
            status_visible: true,
            status: "Ready".to_owned(),
            document: None,
            dirty: false,
            running: false,
            project: None,
            items: Vec::new(),
            selected: None,
            design: false,
            output: Vec::new(),
        }
    }
}

impl IdeState {
    /// The initial state, with `dark` selecting the starting palette.
    pub fn new(dark: bool) -> IdeState {
        IdeState {
            dark,
            ..IdeState::default()
        }
    }

    /// Reduces one message into the state and reports what the window must do.
    pub fn apply(&mut self, msg: &Msg) -> Effect {
        let mut effect = Effect::default();
        match msg {
            Msg::Exit | Msg::AutoClose => effect.quit = true,
            Msg::NewProject => {
                self.status("New project (stub)");
                self.push_output("New Project is not implemented in this prototype");
                effect.status_changed = true;
                effect.output_changed = true;
            }
            Msg::OpenProject => {
                self.status("Open Project");
                self.push_output("Open Project: launch the IDE with a project directory");
                effect.status_changed = true;
                effect.output_changed = true;
            }
            Msg::SelectExplorer(index) => {
                if let Some(item) = self.items.get(*index).cloned() {
                    self.selected = Some(*index);
                    let kind = item.kind;
                    let name = item.name.clone();
                    match kind {
                        ItemKind::Form => {
                            self.design = true;
                            self.status(&format!("Design: {name}"));
                        }
                        ItemKind::Source => {
                            self.design = false;
                            self.status(&format!("Edit: {name}"));
                        }
                    }
                    effect.status_changed = true;
                    effect.design_changed = true;
                }
            }
            Msg::ToggleDesign => {
                self.design = !self.design;
                self.status(if self.design {
                    "Design mode on"
                } else {
                    "Design mode off"
                });
                effect.status_changed = true;
                effect.design_changed = true;
            }
            Msg::DesignSelection(selected) => {
                let line = match selected {
                    Some(index) => format!("Selected control {index}"),
                    None => "Cleared the selection".to_owned(),
                };
                self.push_output(&line);
                effect.output_changed = true;
            }
            Msg::DesignEdited => {
                self.push_output("Form edited");
                effect.output_changed = true;
            }
            Msg::AddControl(kind) => {
                self.status(&format!("Add {kind:?}"));
                self.push_output(&format!("Add {kind:?}"));
                effect.status_changed = true;
                effect.output_changed = true;
            }
            // The property write itself happens in `IdeApp::update`, which is
            // the only place that can reach the live designer; the reducer just
            // asks the status bar to follow.
            Msg::Commit(_) | Msg::SetAnchor(_) => effect.status_changed = true,
            Msg::ResizeFormToPane => {
                self.status("Resize form to pane");
                self.push_output("Resized the form to the design pane");
                effect.status_changed = true;
                effect.output_changed = true;
            }
            Msg::Save => {
                self.status("Saved");
                effect.status_changed = true;
            }
            Msg::Reload => {
                self.status("Reloaded");
                effect.status_changed = true;
            }
            // The dirty flag is recomputed from the live control and baseline in
            // `IdeApp::update`; the reducer only asks for a status-bar refresh.
            Msg::DocumentChanged => effect.status_changed = true,
            Msg::Undo => {
                self.status("Undo");
                effect.status_changed = true;
            }
            Msg::Redo => {
                self.status("Redo");
                effect.status_changed = true;
            }
            Msg::Cut => {
                self.status("Cut");
                effect.status_changed = true;
            }
            Msg::Copy => {
                self.status("Copy");
                effect.status_changed = true;
            }
            Msg::Paste => {
                self.status("Paste");
                effect.status_changed = true;
            }
            Msg::ToggleToolbar => {
                self.toolbar_visible = !self.toolbar_visible;
                effect.layout_changed = true;
                self.status(if self.toolbar_visible {
                    "Toolbar shown"
                } else {
                    "Toolbar hidden"
                });
                effect.status_changed = true;
            }
            Msg::ToggleStatusBar => {
                self.status_visible = !self.status_visible;
                effect.layout_changed = true;
                self.status(if self.status_visible {
                    "Status bar shown"
                } else {
                    "Status bar hidden"
                });
                effect.status_changed = true;
            }
            Msg::LightTheme => effect.theme_changed = self.set_dark(false),
            Msg::DarkTheme => effect.theme_changed = self.set_dark(true),
            Msg::ToggleTheme => effect.theme_changed = self.set_dark(!self.dark),
            Msg::Build => {
                self.running = false;
                self.status("Build requested (stub)");
                self.push_output("Build requested (stub)");
                effect.status_changed = true;
                effect.output_changed = true;
            }
            Msg::Run => {
                self.running = true;
                self.status("Running (stub)");
                self.push_output("Run requested (stub)");
                effect.status_changed = true;
                effect.output_changed = true;
            }
            Msg::Stop => {
                self.running = false;
                self.status("Stopped");
                self.push_output("Stop requested");
                effect.status_changed = true;
                effect.output_changed = true;
            }
            Msg::About => {
                self.status("VisualRust IDE — prototype");
                effect.status_changed = true;
            }
        }
        effect
    }

    /// Records the loaded project's name for the status line and output.
    pub fn set_project(&mut self, name: impl Into<String>) {
        self.project = Some(name.into());
    }

    /// Replaces the explorer rows.
    pub fn set_items(&mut self, items: Vec<ExplorerItem>) {
        self.items = items;
    }

    /// Records the open document's name for the status line.
    pub fn set_document(&mut self, name: impl Into<String>) {
        self.document = Some(name.into());
    }

    /// Sets whether the editor has unsaved changes.
    pub fn set_dirty(&mut self, dirty: bool) {
        self.dirty = dirty;
    }

    /// Overrides the status message, e.g. with the outcome of a save.
    pub fn set_status(&mut self, text: &str) {
        self.status(text);
    }

    /// Appends one line to the output log.
    pub fn push_output(&mut self, line: &str) {
        self.output.push(line.to_owned());
    }

    /// The whole output log, one entry per line.
    pub fn output_text(&self) -> String {
        self.output.join("\r\n")
    }

    /// The full status-bar line: a dirty marker and the document name, then the
    /// last message. Falls back to the bare message when no document is open.
    pub fn status_line(&self) -> String {
        match &self.document {
            Some(name) => format!(
                "{} {name}  |  {}",
                if self.dirty { "[*]" } else { "[ ]" },
                self.status
            ),
            None => self.status.clone(),
        }
    }

    /// Sets the palette, returning whether it changed; the status line follows.
    fn set_dark(&mut self, dark: bool) -> bool {
        let changed = self.dark != dark;
        self.dark = dark;
        self.status(if dark { "Theme: dark" } else { "Theme: light" });
        changed
    }

    fn status(&mut self, text: &str) {
        self.status = text.to_owned();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn item(name: &str, kind: ItemKind) -> ExplorerItem {
        ExplorerItem {
            name: name.to_owned(),
            path: Some(PathBuf::from(name)),
            kind,
        }
    }

    #[test]
    fn toggling_the_theme_flips_dark_once() {
        let mut state = IdeState::new(true);
        let first = state.apply(&Msg::ToggleTheme);
        assert!(first.theme_changed);
        assert!(!state.dark);
        assert_eq!(state.status, "Theme: light");
        // Selecting the already-current palette does not ask for a re-theme.
        let second = state.apply(&Msg::LightTheme);
        assert!(!second.theme_changed);
    }

    #[test]
    fn bar_toggles_change_the_layout() {
        let mut state = IdeState::default();
        let effect = state.apply(&Msg::ToggleToolbar);
        assert!(effect.layout_changed);
        assert!(!state.toolbar_visible);
        let effect = state.apply(&Msg::ToggleStatusBar);
        assert!(effect.layout_changed);
        assert!(!state.status_visible);
    }

    #[test]
    fn exit_quits_and_non_layout_messages_do_not() {
        let mut state = IdeState::default();
        assert!(state.apply(&Msg::Exit).quit);
        let effect = state.apply(&Msg::About);
        assert!(!effect.quit && !effect.layout_changed && !effect.theme_changed);
        assert!(effect.status_changed);
    }

    #[test]
    fn status_line_reflects_the_dirty_flag() {
        let mut state = IdeState::default();
        assert_eq!(state.status_line(), "Ready");
        state.set_document("main.dyon");
        let clean = state.status_line();
        assert!(clean.contains("main.dyon") && clean.contains("[ ]"));
        state.set_dirty(true);
        assert!(state.status_line().contains("[*]"));
    }

    #[test]
    fn a_document_change_asks_for_a_status_refresh() {
        let mut state = IdeState::default();
        let effect = state.apply(&Msg::DocumentChanged);
        assert!(effect.status_changed);
        assert!(!effect.quit && !effect.layout_changed && !effect.theme_changed);
    }

    #[test]
    fn run_and_stop_track_the_runner() {
        let mut state = IdeState::default();
        assert!(!state.running);
        let _ = state.apply(&Msg::Run);
        assert!(state.running);
        let _ = state.apply(&Msg::Stop);
        assert!(!state.running);
    }

    #[test]
    fn selecting_a_form_enters_design_and_a_source_leaves_it() {
        let mut state = IdeState::default();
        state.set_items(vec![
            item("app.vrform", ItemKind::Form),
            item("main.dyon", ItemKind::Source),
        ]);
        let effect = state.apply(&Msg::SelectExplorer(0));
        assert!(state.design && effect.design_changed);
        let effect = state.apply(&Msg::SelectExplorer(1));
        assert!(!state.design && effect.design_changed);
        // An out-of-range index changes nothing.
        let effect = state.apply(&Msg::SelectExplorer(9));
        assert!(!effect.design_changed && !effect.status_changed);
    }

    #[test]
    fn toggle_design_flips_the_flag_and_reports_it() {
        let mut state = IdeState::default();
        let effect = state.apply(&Msg::ToggleDesign);
        assert!(state.design && effect.design_changed);
        let effect = state.apply(&Msg::ToggleDesign);
        assert!(!state.design && effect.design_changed);
    }

    #[test]
    fn run_appends_to_the_output_log() {
        let mut state = IdeState::default();
        let effect = state.apply(&Msg::Run);
        assert!(effect.output_changed);
        assert!(state.output_text().contains("Run requested"));
    }
}
