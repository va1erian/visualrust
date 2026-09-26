//! The shell's message vocabulary.

/// One user intent, raised by the window chrome and delivered to
/// [`IdeApp::update`](crate::IdeApp).
///
/// `Msg` is deliberately free of `xui` types: menus, toolbar buttons,
/// accelerators and timers all translate into this enum, and
/// [`IdeState::apply`](crate::IdeState::apply) reduces it without touching a
/// window. That keeps the view model unit-testable on a headless machine and
/// keeps the list of things the shell can do in one place.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Msg {
    /// Start a new project.
    NewProject,
    /// Open an existing project.
    OpenProject,
    /// Save the active document.
    Save,
    /// Reload the active document from disk.
    Reload,
    /// The editor's text or save point changed; recompute the dirty flag.
    DocumentChanged,
    /// Close the window.
    Exit,
    /// Undo the last edit.
    Undo,
    /// Redo the last undone edit.
    Redo,
    /// Cut the selection.
    Cut,
    /// Copy the selection.
    Copy,
    /// Paste the clipboard.
    Paste,
    /// Show or hide the toolbar.
    ToggleToolbar,
    /// Show or hide the status bar.
    ToggleStatusBar,
    /// Switch to the light palette.
    LightTheme,
    /// Switch to the dark palette.
    DarkTheme,
    /// Flip between the light and dark palettes.
    ToggleTheme,
    /// Build the active project.
    Build,
    /// Run the active project.
    Run,
    /// Stop the running project.
    Stop,
    /// Show the about text.
    About,
    /// The autoclose timer fired; leave the message loop.
    AutoClose,
}
