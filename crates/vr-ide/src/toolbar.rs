//! The shell's toolbar, mapped to [`Msg`](crate::Msg).

use win32ui::prelude::*;

use crate::Msg;

/// The toolbar's buttons, in order.
///
/// Shortcut text is display-only on a toolbar button; the menu bar registers
/// the same accelerators, so a click and a key press raise the same `Msg`.
pub(crate) fn items() -> Vec<ToolbarItem<Msg>> {
    vec![
        ToolbarItem::new("New")
            .with_icon(ToolbarIcon::Circle)
            .tooltip("New project")
            .shortcut(Shortcut::ctrl(Key::N))
            .on_click(|| Some(Msg::NewProject)),
        ToolbarItem::new("Open")
            .with_icon(ToolbarIcon::Arrow)
            .tooltip("Open project")
            .shortcut(Shortcut::ctrl(Key::O))
            .on_click(|| Some(Msg::OpenProject)),
        ToolbarItem::new("Save")
            .with_icon(ToolbarIcon::Check)
            .tooltip("Save")
            .shortcut(Shortcut::ctrl(Key::S))
            .on_click(|| Some(Msg::Save)),
        ToolbarItem::new("Build")
            .with_icon(ToolbarIcon::Chevron)
            .tooltip("Build project")
            .shortcut(Shortcut::ctrl(Key::B))
            .on_click(|| Some(Msg::Build)),
        ToolbarItem::new("Run")
            .with_icon(ToolbarIcon::Arrow)
            .tooltip("Run project")
            .shortcut(Shortcut::key(Key::F5))
            .on_click(|| Some(Msg::Run)),
        ToolbarItem::new("Stop")
            .with_icon(ToolbarIcon::Close)
            .tooltip("Stop")
            .shortcut(Shortcut::shift(Key::F5))
            .on_click(|| Some(Msg::Stop)),
        ToolbarItem::new("Theme")
            .with_icon(ToolbarIcon::Circle)
            .tooltip("Toggle light/dark")
            .shortcut(Shortcut::ctrl(Key::T))
            .on_click(|| Some(Msg::ToggleTheme)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_button_maps_to_a_message() {
        let items = items();
        assert_eq!(items.len(), 7);
    }
}
