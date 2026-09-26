//! The IDE view model.
//!
//! The shell is a small unidirectional loop:
//!
//! ```text
//!   chrome (menus / toolbar / accelerators / timer)
//!         |  Msg
//!         v
//!   IdeState::apply     pure reducer, no xui types
//!         |  Effect { quit, theme_changed, layout_changed, status_changed }
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
            Msg::NewProject => self.status("New project"),
            Msg::OpenProject => self.status("Open project"),
            Msg::Save => self.status("Saved"),
            Msg::Reload => self.status("Reloaded"),
            // The dirty flag is recomputed from the live control and baseline in
            // `IdeApp::update`; the reducer only asks for a status-bar refresh.
            Msg::DocumentChanged => {}
            Msg::Undo => self.status("Undo"),
            Msg::Redo => self.status("Redo"),
            Msg::Cut => self.status("Cut"),
            Msg::Copy => self.status("Copy"),
            Msg::Paste => self.status("Paste"),
            Msg::ToggleToolbar => {
                self.toolbar_visible = !self.toolbar_visible;
                effect.layout_changed = true;
                self.status(if self.toolbar_visible {
                    "Toolbar shown"
                } else {
                    "Toolbar hidden"
                });
            }
            Msg::ToggleStatusBar => {
                self.status_visible = !self.status_visible;
                effect.layout_changed = true;
                self.status(if self.status_visible {
                    "Status bar shown"
                } else {
                    "Status bar hidden"
                });
            }
            Msg::LightTheme => effect.theme_changed = self.set_dark(false),
            Msg::DarkTheme => effect.theme_changed = self.set_dark(true),
            Msg::ToggleTheme => effect.theme_changed = self.set_dark(!self.dark),
            Msg::Build => {
                self.running = false;
                self.status("Build requested");
            }
            Msg::Run => {
                self.running = true;
                self.status("Running");
            }
            Msg::Stop => {
                self.running = false;
                self.status("Stopped");
            }
            Msg::About => self.status("VisualRust IDE — M1 shell"),
        }
        effect.status_changed = matches!(
            msg,
            Msg::NewProject
                | Msg::OpenProject
                | Msg::Save
                | Msg::Reload
                | Msg::DocumentChanged
                | Msg::Undo
                | Msg::Redo
                | Msg::Cut
                | Msg::Copy
                | Msg::Paste
                | Msg::ToggleToolbar
                | Msg::ToggleStatusBar
                | Msg::LightTheme
                | Msg::DarkTheme
                | Msg::ToggleTheme
                | Msg::Build
                | Msg::Run
                | Msg::Stop
                | Msg::About
        );
        effect
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
}
