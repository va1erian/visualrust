//! The [`IdeApp`]: builds the shell's widgets and applies each [`Msg`].

use xui::prelude::*;

use crate::msg::Msg;
use crate::state::IdeState;
use crate::{menus, toolbar};

/// Environment variable that makes the window quit itself, in milliseconds.
pub(crate) const AUTOCLOSE_ENV: &str = "xui_DEMO_AUTOCLOSE_MS";
/// Environment variable that selects the initial palette (`light` or `dark`).
pub(crate) const THEME_ENV: &str = "xui_DEMO_THEME";
/// The window title; the smoke test selects the window by it.
pub const TITLE: &str = "VisualRust IDE";

/// Welcome text in the central placeholder.
const WELCOME: &str = "VisualRust IDE\n\nFile \u{25b8} New Project (Ctrl+N) to begin";

/// The IDE shell's application state.
///
/// It holds the view model plus the widgets whose `HWND`s must outlive
/// `run_app`; `update` is the only place they are touched.
pub struct IdeApp {
    state: IdeState,
    /// `None` only if the control could not be created; the shell still runs.
    toolbar: Option<Toolbar<Msg>>,
    /// Owns the placeholder's `HWND` for the window's lifetime; it paints
    /// itself and is never read again.
    #[allow(dead_code)]
    editor: Option<Label>,
    status: Option<StatusBar<Msg>>,
}

impl IdeApp {
    /// The window spec's palette, from `xui_DEMO_THEME` (default dark).
    pub(crate) fn initial_theme() -> Theme {
        match std::env::var(THEME_ENV) {
            Ok(value) if value.eq_ignore_ascii_case("light") => Theme::light(),
            _ => Theme::dark(),
        }
    }

    /// Builds the shell's widgets and installs its layout. This is the closure
    /// `run_app` calls once the window exists.
    pub(crate) fn build(ui: &mut Ui<Msg>) -> IdeApp {
        let mut state = IdeState::new(ui.theme().is_dark);
        let mut failures: Vec<String> = Vec::new();

        let toolbar = match Toolbar::new(ui, toolbar::items()) {
            Ok(bar) => Some(bar),
            Err(error) => {
                failures.push(format!("toolbar: {error}"));
                None
            }
        };
        let editor = match Label::new(ui, Rect::new(0, 0, 480, 96), WELCOME) {
            Ok(label) => Some(label),
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

        if !failures.is_empty() {
            state.status = failures.join("; ");
        }
        if let Some(bar) = &status {
            bar.set_parts(&[-1]);
            bar.set_text(0, &state.status);
        }

        ui.set_menu_bar(menus::menu_bar(&state));
        ui.set_layout(layout(&toolbar, &editor, &status));
        arm_autoclose(ui);

        IdeApp {
            state,
            toolbar,
            editor,
            status,
        }
    }
}

impl App for IdeApp {
    type Msg = Msg;

    fn update(&mut self, msg: Msg, ui: &mut Ui<Msg>) {
        let effect = self.state.apply(&msg);
        if effect.theme_changed {
            ui.set_theme(palette(self.state.dark));
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
            bar.set_text(0, &self.state.status);
        }
        if effect.quit {
            ui.quit();
        }
    }
}

/// A palette for the window; `dark` selects [`Theme::dark`].
fn palette(dark: bool) -> Theme {
    if dark { Theme::dark() } else { Theme::light() }
}

/// The window's column: toolbar, then the editor placeholder (which absorbs
/// the slack), then the status bar. A missing bar simply does not take a slot.
fn layout(
    toolbar: &Option<Toolbar<Msg>>,
    editor: &Option<Label>,
    status: &Option<StatusBar<Msg>>,
) -> Layout {
    let mut layout = Layout::column();
    if let Some(bar) = toolbar {
        layout = layout.item(bar);
    }
    match editor {
        Some(label) => layout = layout.item(label.fill(1)),
        // An empty nested layout still fills, keeping the status bar at the
        // bottom when the placeholder could not be created.
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
