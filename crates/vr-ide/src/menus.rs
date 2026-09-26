//! The shell's menu bar, mapped to [`Msg`](crate::Msg).

use xui::prelude::*;

use crate::Msg;

/// Key of the "Light" palette radio, so `update` can tick it on a theme change.
pub(crate) const LIGHT: &str = "theme_light";
/// Key of the "Dark" palette radio.
pub(crate) const DARK: &str = "theme_dark";
/// Key of the "Toolbar" checked item.
pub(crate) const TOOLBAR: &str = "view_toolbar";
/// Key of the "Status bar" checked item.
pub(crate) const STATUS: &str = "view_status";

/// Builds the File / Edit / View / Build / Run / Help menu bar.
///
/// `state` supplies the radio ticks and the two bar checkmarks, so the bar
/// opens in the same state the window is in. Every enabled item's shortcut is
/// registered as an accelerator by `Ui::set_menu_bar`, so the menu and the
/// keyboard can never drift apart.
pub(crate) fn menu_bar(state: &crate::IdeState) -> Menu<Msg> {
    Menu::new()
        .submenu("&File", file())
        .submenu("&Edit", edit())
        .submenu("&View", view(state))
        .submenu("&Build", build())
        .submenu("&Run", run())
        .submenu("&Help", help())
}

fn file() -> Menu<Msg> {
    Menu::new()
        .item("&New Project", Shortcut::ctrl(Key::N), || Msg::NewProject)
        .item("&Open Project…", Shortcut::ctrl(Key::O), || {
            Msg::OpenProject
        })
        .item("&Save", Shortcut::ctrl(Key::S), || Msg::Save)
        .separator()
        .item("E&xit", Shortcut::ctrl(Key::Q), || Msg::Exit)
}

fn edit() -> Menu<Msg> {
    Menu::new()
        .item("&Undo", Shortcut::ctrl(Key::Z), || Msg::Undo)
        .item("&Redo", Shortcut::ctrl(Key::Y), || Msg::Redo)
        .separator()
        .item("Cu&t", Shortcut::ctrl(Key::X), || Msg::Cut)
        .item("&Copy", Shortcut::ctrl(Key::C), || Msg::Copy)
        .item("&Paste", Shortcut::ctrl(Key::V), || Msg::Paste)
}

fn view(state: &crate::IdeState) -> Menu<Msg> {
    Menu::new()
        .radio_item("&Light", None, !state.dark, || Msg::LightTheme)
        .keyed(LIGHT)
        .radio_item("&Dark", None, state.dark, || Msg::DarkTheme)
        .keyed(DARK)
        .separator()
        .checked_item("&Toolbar", None, state.toolbar_visible, || {
            Msg::ToggleToolbar
        })
        .keyed(TOOLBAR)
        .checked_item("&Status Bar", None, state.status_visible, || {
            Msg::ToggleStatusBar
        })
        .keyed(STATUS)
        .separator()
        .item("Toggle &Theme", Shortcut::ctrl(Key::T), || Msg::ToggleTheme)
}

fn build() -> Menu<Msg> {
    Menu::new().item("&Build Project", Shortcut::ctrl(Key::B), || Msg::Build)
}

fn run() -> Menu<Msg> {
    Menu::new()
        .item("&Run", Shortcut::key(Key::F5), || Msg::Run)
        .item("&Stop", Shortcut::shift(Key::F5), || Msg::Stop)
}

fn help() -> Menu<Msg> {
    Menu::new().item("&About VisualRust", None, || Msg::About)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The bar is pure data: building it needs no window, so this pins the
    /// builder against accidental panics on a headless machine.
    #[test]
    fn menu_bar_builds_without_a_window() {
        let _menu = menu_bar(&crate::IdeState::default());
    }
}
