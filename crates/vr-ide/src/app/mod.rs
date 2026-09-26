//! The [`IdeApp`]: builds the prototype's widgets and applies each [`Msg`].

use std::path::{Path, PathBuf};

use vr_forms::Form;
use vr_scintilla::{ScintillaHost, Scn};
use xui::prelude::*;

use crate::designer::{self, DesignPane};
use crate::document::{self, Document};
use crate::explorer::{self, ItemKind};
use crate::layout;
use crate::msg::Msg;
use crate::output::OutputPane;
use crate::state::IdeState;
use crate::{menus, toolbar};

/// Environment variable that makes the window quit itself, in milliseconds.
pub(crate) const AUTOCLOSE_ENV: &str = "xui_DEMO_AUTOCLOSE_MS";
/// Environment variable that selects the initial palette (`light` or `dark`).
pub(crate) const THEME_ENV: &str = "xui_DEMO_THEME";
/// Environment variable that starts the shell in design mode (`1`), so a
/// capture test can evidence the designer without driving input.
pub(crate) const DESIGN_ENV: &str = "VR_IDE_PROTOTYPE_DESIGN";
/// The output pane's starting height, in design units.
const OUTPUT_HEIGHT: f32 = 150.0;
/// The window's starting client height, so the split divider opens near the
/// output pane's top on a fresh window.
const INITIAL_CLIENT_HEIGHT: f32 = 690.0;
/// The window title; the smoke test selects the window by it.
pub const TITLE: &str = "VisualRust IDE";

/// The IDE shell's application state.
///
/// It holds the view model, the project, the widgets whose `HWND`s must outlive
/// `run_app`, and the active designer. `update` is the only place they are
/// touched except for the one-time load in `build`.
pub struct IdeApp {
    state: IdeState,
    /// `None` only if the control could not be created; the shell still runs.
    toolbar: Option<Toolbar<Msg>>,
    /// The project explorer; `None` when the tree could not be created. Kept
    /// alive for its `HWND` even though the shell never queries it again: its
    /// selection arrives as [`Msg::SelectExplorer`].
    #[allow(dead_code)]
    explorer: Option<TreeView<usize, Msg>>,
    /// The editor pane: a real Scintilla control filling the central area.
    /// `None` when the host could not be created (a session without a desktop).
    editor: Option<ScintillaHost<Msg>>,
    /// The bottom output pane; `None` when it could not be created.
    output: Option<OutputPane>,
    /// The central form pane, shown instead of the editor in design mode.
    design: Option<DesignPane>,
    status: Option<StatusBar<Msg>>,
    /// The loaded project, for resolving an explorer selection.
    project: explorer::Loaded,
    /// The form currently previewed in design mode.
    selected_form: Option<Form>,
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

    /// Builds the prototype's widgets and installs its layout.
    ///
    /// `path` is the CLI argument: a project directory, a file to open, or
    /// nothing. `cwd` is the fallback project location.
    pub(crate) fn build(ui: &mut Ui<Msg>, path: Option<PathBuf>, cwd: PathBuf) -> IdeApp {
        let mut state = IdeState::new(ui.theme().is_dark);
        let mut failures: Vec<String> = Vec::new();

        // A directory argument is the project root; a file argument opens that
        // document and uses its parent as the root; otherwise the cwd is used.
        let (start, file_arg) = split_argument(path, &cwd);
        let project = explorer::load(&start);
        let initial = document::initial(file_arg, &project.root);
        let baseline = initial.document.text().to_owned();

        state.set_project(&project.name);
        state.set_items(project.items.clone());
        state.set_document(initial.document.name());
        state.push_output(&project.note);
        if let Some(note) = &initial.note {
            state.set_status(note);
            state.push_output(note);
        }

        let toolbar = match Toolbar::new(ui, toolbar::items()) {
            Ok(bar) => Some(bar),
            Err(error) => {
                failures.push(format!("toolbar: {error}"));
                None
            }
        };
        let explorer = match TreeView::new(ui, explorer::ExplorerModel::new(project.items.clone()))
        {
            Ok(tree) => Some(tree.on_select(|index| Some(Msg::SelectExplorer(*index)))),
            Err(error) => {
                failures.push(format!("explorer: {error}"));
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
        let output = match OutputPane::new(ui) {
            Ok(pane) => Some(pane),
            Err(error) => {
                failures.push(format!("output: {error}"));
                None
            }
        };
        let design = match DesignPane::new(ui) {
            Ok(pane) => Some(pane),
            Err(error) => {
                failures.push(format!("design pane: {error}"));
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

        if !failures.is_empty() {
            let joined = failures.join("; ");
            state.set_status(&joined);
            state.push_output(&format!("Unavailable: {joined}"));
        }

        ui.set_menu_bar(menus::menu_bar(&state));
        ui.set_layout(layout::build(
            &toolbar,
            &explorer,
            &editor,
            &output,
            &design,
            &status,
            dip(INITIAL_CLIENT_HEIGHT - OUTPUT_HEIGHT - 60.0),
        ));
        arm_autoclose(ui);

        let mut app = IdeApp {
            state,
            toolbar,
            explorer,
            editor,
            output,
            design,
            status,
            project,
            selected_form: None,
            document: initial.document,
            baseline,
        };
        app.sync_bars(ui);
        app.refresh_output();
        app.apply_scintilla_theme(ui);
        // The design pane starts hidden; the editor is the default centre.
        if let Some(pane) = &app.design {
            pane.set_visible(app.state.design);
        }
        if env_flag(DESIGN_ENV) {
            app.state.design = true;
            app.sync_design(ui);
        }
        app
    }

    /// Applies the current palette to the Scintilla control, which xui's
    /// themed-children pass cannot reach.
    fn apply_scintilla_theme(&self, ui: &Ui<Msg>) {
        if let Some(host) = &self.editor {
            host.apply_theme(&ui.theme());
        }
    }

    /// Pushes the current bar visibility and status text into the widgets.
    fn sync_bars(&self, ui: &Ui<Msg>) {
        if let Some(bar) = &self.toolbar {
            bar.set_visible(self.state.toolbar_visible);
        }
        if let Some(bar) = &self.status {
            bar.set_visible(self.state.status_visible);
            bar.set_parts(&[-1]);
            bar.set_text(0, &self.state.status_line());
        }
        ui.set_menu_checked(menus::TOOLBAR, self.state.toolbar_visible);
        ui.set_menu_checked(menus::STATUS, self.state.status_visible);
        ui.set_menu_checked(menus::DESIGN, self.state.design);
    }

    /// Writes the log to the output pane.
    fn refresh_output(&self) {
        if let Some(pane) = &self.output {
            pane.set_lines(&self.state.output);
        }
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

    /// Loads the selected explorer item: a form becomes the designer's form, a
    /// source is opened in the editor.
    fn apply_selection(&mut self) {
        let Some(index) = self.state.selected else {
            return;
        };
        // `as_slice` disambiguates `get` from the `GridModel`/`ListModel` impls
        // that the xui prelude also brings into scope for `Vec<T>`.
        let Some(item) = self.project.items.as_slice().get(index).cloned() else {
            return;
        };
        match item.kind {
            ItemKind::Form => {
                self.selected_form = Some(self.load_form(item.path.as_deref()));
            }
            ItemKind::Source => self.open_source(item.path.as_deref(), &item.name),
        }
    }

    /// Reads a `.vrform`, falling back to the built-in sample with a note.
    fn load_form(&mut self, path: Option<&Path>) -> Form {
        match path {
            Some(path) => match Form::load(path) {
                Ok(form) => {
                    self.state
                        .push_output(&format!("Loaded form {}", path.display()));
                    form
                }
                Err(error) => {
                    self.state
                        .push_output(&format!("Form unavailable ({error}); using sample"));
                    explorer::sample_form()
                }
            },
            None => explorer::sample_form(),
        }
    }

    /// Replaces the editor document with `path`, or the built-in sample.
    fn open_source(&mut self, path: Option<&Path>, name: &str) {
        match path {
            Some(path) => match Document::open(path) {
                Ok(document) => {
                    if let Some(host) = &self.editor {
                        host.set_text(document.text());
                    }
                    self.baseline = document.text().to_owned();
                    self.state.set_document(document.name());
                    self.state.set_dirty(false);
                    self.state.push_output(&format!("Opened {name}"));
                    self.document = document;
                }
                Err(error) => {
                    self.state.set_status(&format!("Open failed: {error}"));
                    self.state.push_output(&format!("Open failed: {error}"));
                }
            },
            None => {
                if let Some(host) = &self.editor {
                    host.set_text(explorer::SAMPLE_SOURCE);
                }
                self.baseline = explorer::SAMPLE_SOURCE.to_owned();
                self.state.set_document(name);
                self.state.set_dirty(false);
                self.state.push_output("Showing the built-in Dyon sample");
                self.document = Document::untitled(explorer::SAMPLE_SOURCE);
            }
        }
    }

    /// Enters or leaves design mode so the central pane matches
    /// [`IdeState::design`].
    ///
    /// Design mode is a mode, not a tab: the editor and the form pane share the
    /// central slot, and the editor is hidden while design mode is on. The
    /// pinned xui cannot host the live `DesignerSurface` here — see
    /// [`crate::designer`] — so the pane renders the selected form model.
    fn sync_design(&mut self, ui: &Ui<Msg>) {
        if self.state.design {
            let form = self
                .selected_form
                .clone()
                .unwrap_or_else(explorer::sample_form);
            let text = designer::render(Some(&form));
            if let Some(pane) = &self.design {
                pane.set_text(&text);
                pane.set_visible(true);
            }
            self.state
                .push_output(&format!("Design mode on: {}", form.name));
        } else {
            if let Some(pane) = &self.design {
                pane.set_visible(false);
            }
            self.state.push_output("Design mode off");
        }
        if let Some(host) = &self.editor {
            host.set_visible(!self.state.design);
        }
        ui.set_menu_checked(menus::DESIGN, self.state.design);
        self.refresh_output();
        if let Some(bar) = &self.status {
            bar.set_text(0, &self.state.status_line());
        }
        ui.relayout();
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
            Msg::SelectExplorer(_) => self.apply_selection(),
            _ => {}
        }
        if effect.design_changed {
            self.sync_design(ui);
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
        if effect.output_changed {
            self.refresh_output();
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

/// Splits the CLI argument into a project start directory and an optional file.
fn split_argument(path: Option<PathBuf>, cwd: &Path) -> (PathBuf, Option<PathBuf>) {
    match path {
        Some(path) if path.is_dir() => (path, None),
        Some(path) => {
            let parent = path
                .parent()
                .filter(|dir| !dir.as_os_str().is_empty())
                .map(Path::to_path_buf)
                .unwrap_or_else(|| cwd.to_path_buf());
            (parent, Some(path))
        }
        None => (cwd.to_path_buf(), None),
    }
}

/// Whether an environment variable is set to a truthy value.
fn env_flag(name: &str) -> bool {
    std::env::var(name).is_ok_and(|value| value != "0" && !value.is_empty())
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

#[cfg(test)]
mod tests;
