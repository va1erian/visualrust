//! The [`IdeApp`]: builds the shell's widgets and applies each [`Msg`].

use std::path::PathBuf;

use vr_scintilla::{ScintillaHost, Scn};
use xui::prelude::*;

use crate::document::{self, Document};
use crate::msg::Msg;
use crate::state::IdeState;
use crate::{menus, toolbar};

/// Environment variable that makes the window quit itself, in milliseconds.
pub(crate) const AUTOCLOSE_ENV: &str = "xui_DEMO_AUTOCLOSE_MS";
/// Environment variable that selects the initial palette (`light` or `dark`).
pub(crate) const THEME_ENV: &str = "xui_DEMO_THEME";
/// The window title; the smoke test selects the window by it.
pub const TITLE: &str = "VisualRust IDE";

/// The IDE shell's application state.
///
/// It holds the view model, the document opened into the editor, the text that
/// document was last loaded or saved with, and the widgets whose `HWND`s must
/// outlive `run_app`; `update` is the only place they are touched except for the
/// one-time load in `build`.
pub struct IdeApp {
    state: IdeState,
    /// `None` only if the control could not be created; the shell still runs.
    toolbar: Option<Toolbar<Msg>>,
    /// The editor pane: a real Scintilla control filling the central area.
    /// `None` when the host could not be created (a session without a desktop).
    editor: Option<ScintillaHost<Msg>>,
    status: Option<StatusBar<Msg>>,
    /// The open document; owns the path for save and reload.
    document: Document,
    /// The editor text as last loaded or saved. The live control is compared
    /// against this to decide the dirty indicator, so a notification that only
    /// restyled the text does not mark the document modified.
    baseline: String,
}

impl IdeApp {
    /// The window spec's palette, from `xui_DEMO_THEME` (default dark).
    pub(crate) fn initial_theme() -> Theme {
        match std::env::var(THEME_ENV) {
            Ok(value) if value.eq_ignore_ascii_case("light") => Theme::light(),
            _ => Theme::dark(),
        }
    }

    /// Builds the shell's widgets and installs its layout. `path` is the CLI
    /// document argument, if any; `cwd` anchors the manifest and sample search.
    pub(crate) fn build(ui: &mut Ui<Msg>, path: Option<PathBuf>, cwd: PathBuf) -> IdeApp {
        let mut state = IdeState::new(ui.theme().is_dark);
        let mut failures: Vec<String> = Vec::new();

        let initial = document::initial(path, &cwd);
        let baseline = initial.document.text().to_owned();

        let toolbar = match Toolbar::new(ui, toolbar::items()) {
            Ok(bar) => Some(bar),
            Err(error) => {
                failures.push(format!("toolbar: {error}"));
                None
            }
        };
        let editor = match ScintillaHost::new(ui, map_scn) {
            Ok(host) => {
                // The control styles itself from this text through the container
                // lexer; the app only decides what it opens with.
                host.set_text(&baseline);
                Some(host)
            }
            Err(error) => {
                failures.push(format!("editor: {error}"));
                None
            }
        };
        let status = match StatusBar::new(ui) {
            Ok(bar) => Some(bar),
            Err(error) => {
                failures.push(format!("status bar: {error}"));
                None
            }
        };

        state.set_document(initial.document.name());
        if let Some(note) = initial.note {
            state.set_status(&note);
        }
        if !failures.is_empty() {
            state.set_status(&failures.join("; "));
        }
        if let Some(bar) = &status {
            bar.set_parts(&[-1]);
            bar.set_text(0, &state.status_line());
        }

        ui.set_menu_bar(menus::menu_bar(&state));
        ui.set_layout(layout(&toolbar, &editor, &status));
        arm_autoclose(ui);

        IdeApp {
            state,
            toolbar,
            editor,
            status,
            document: initial.document,
            baseline,
        }
        .with_theme(ui)
    }

    /// Re-applies the current palette to the hosted control, which xui's
    /// themed-children pass cannot reach, then returns `self`.
    fn with_theme(self, ui: &Ui<Msg>) -> IdeApp {
        if let Some(host) = &self.editor {
            host.apply_theme(&ui.theme());
        }
        self
    }

    /// Writes the live editor text to the document's file and clears the dirty
    /// flag; failures land in the status bar instead.
    fn save(&mut self) {
        let text = match self.editor.as_ref() {
            Some(host) => match host.text() {
                Ok(text) => text,
                Err(error) => {
                    self.state.set_status(&format!("Save failed: {error}"));
                    return;
                }
            },
            None => {
                self.state.set_status("No editor to save");
                return;
            }
        };
        match self.document.write(&text) {
            Ok(()) => {
                self.baseline = text;
                self.state.set_dirty(false);
                let name = self.document.name().to_owned();
                self.state.set_status(&format!("Saved {name}"));
            }
            Err(error) => self.state.set_status(&format!("Save failed: {error}")),
        }
    }

    /// Re-reads the document's file and replaces the editor text with it.
    fn reload(&mut self) {
        if let Err(error) = self.document.reload() {
            self.state.set_status(&format!("Reload failed: {error}"));
            return;
        }
        if let Some(host) = &self.editor {
            host.set_text(self.document.text());
        }
        self.baseline = self.document.text().to_owned();
        self.state.set_dirty(false);
        let name = self.document.name().to_owned();
        self.state.set_status(&format!("Reloaded {name}"));
    }

    /// Recomputes the dirty flag by comparing the live text with the baseline,
    /// so style-only notifications (which also arrive as modifications) do not
    /// light the indicator.
    fn refresh_dirty(&mut self) {
        let Some(host) = &self.editor else {
            return;
        };
        if let Ok(text) = host.text() {
            self.state.set_dirty(text != self.baseline);
        }
    }
}

impl App for IdeApp {
    type Msg = Msg;

    fn update(&mut self, msg: Msg, ui: &mut Ui<Msg>) {
        let effect = self.state.apply(&msg);
        match msg {
            Msg::Save => self.save(),
            Msg::Reload => self.reload(),
            Msg::DocumentChanged => self.refresh_dirty(),
            _ => {}
        }
        if effect.theme_changed {
            ui.set_theme(palette(self.state.dark));
            // The hosted control is not a xui child, so re-apply its palette
            // explicitly after the window switched.
            if let Some(host) = &self.editor {
                host.apply_theme(&ui.theme());
            }
            ui.set_menu_checked(menus::LIGHT, !self.state.dark);
            ui.set_menu_checked(menus::DARK, self.state.dark);
        }
        if effect.layout_changed {
            if let Some(bar) = &self.toolbar {
                bar.set_visible(self.state.toolbar_visible);
            }
            if let Some(bar) = &self.status {
                bar.set_visible(self.state.status_visible);
            }
            ui.set_menu_checked(menus::TOOLBAR, self.state.toolbar_visible);
            ui.set_menu_checked(menus::STATUS, self.state.status_visible);
            ui.relayout();
        }
        if effect.status_changed
            && let Some(bar) = &self.status
        {
            bar.set_text(0, &self.state.status_line());
        }
        if effect.quit {
            ui.quit();
        }
    }
}

/// Maps a Scintilla notification to the app's message, or `None` to ignore it.
///
/// A modification and both save-point transitions all ask for a dirty
/// recomputation. Scintilla also raises `SCN_MODIFIED` for style changes, which
/// is why the dirty flag is a text comparison rather than "was modified".
fn map_scn(scn: Scn) -> Option<Msg> {
    match scn {
        Scn::Modified { .. } | Scn::SavePointLeft | Scn::SavePointReached => {
            Some(Msg::DocumentChanged)
        }
        _ => None,
    }
}

/// A palette for the window; `dark` selects [`Theme::dark`].
fn palette(dark: bool) -> Theme {
    if dark { Theme::dark() } else { Theme::light() }
}

/// The window's column: toolbar, then the editor pane (which absorbs the
/// slack), then the status bar. A missing bar simply does not take a slot.
fn layout(
    toolbar: &Option<Toolbar<Msg>>,
    editor: &Option<ScintillaHost<Msg>>,
    status: &Option<StatusBar<Msg>>,
) -> Layout {
    let mut layout = Layout::column();
    if let Some(bar) = toolbar {
        layout = layout.item(bar);
    }
    match editor {
        Some(host) => layout = layout.item(host.fill(1)),
        // An empty nested layout still fills, keeping the status bar at the
        // bottom when the editor could not be created.
        None => layout = layout.item(Layout::column()),
    }
    if let Some(bar) = status {
        layout = layout.item(bar);
    }
    layout
}

/// Arms `xui_DEMO_AUTOCLOSE_MS` so a headless run leaves on its own.
fn arm_autoclose(ui: &mut Ui<Msg>) {
    let Some(millis) = std::env::var(AUTOCLOSE_ENV)
        .ok()
        .and_then(|value| value.parse::<u32>().ok())
    else {
        return;
    };
    let Ok(timer) = ui.set_timer(millis) else {
        return;
    };
    ui.on_timer(move |fired| (fired == timer).then_some(Msg::AutoClose));
}
