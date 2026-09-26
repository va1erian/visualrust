//! The shell's message vocabulary.

use vr_forms::Anchor;

/// A widget family the Form menu can add to the hosted form.
///
/// It is a plain, `Copy` mirror of the [`vr_forms::ControlKind`] variants the
/// palette exposes, kept here so [`Msg`] stays tiny and `Eq` (the model's kinds
/// carry `f64` settings and cannot be). [`crate::designer::control_kind`] turns
/// one into a concrete model kind with usable defaults.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaletteKind {
    Button,
    Label,
    Edit,
    CheckBox,
    ComboBox,
    ProgressBar,
    Slider,
    GroupBox,
    RadioGroup,
}

/// One editable field of the property inspector.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PropertyField {
    Name,
    Text,
    X,
    Y,
    Width,
    Height,
    Enabled,
    Visible,
}

impl PropertyField {
    /// The model property name [`vr_forms::design::FormDesigner::set_property`]
    /// and the `.vrform` key understand.
    pub const fn name(self) -> &'static str {
        match self {
            PropertyField::Name => "name",
            PropertyField::Text => "text",
            PropertyField::X => "x",
            PropertyField::Y => "y",
            PropertyField::Width => "width",
            PropertyField::Height => "height",
            PropertyField::Enabled => "enabled",
            PropertyField::Visible => "visible",
        }
    }
}

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
    /// A project explorer item was selected, by its index in the flat list.
    SelectExplorer(usize),
    /// Enter or leave the form-design mode.
    ToggleDesign,
    /// The hosted designer selected a control.
    DesignSelection(Option<usize>),
    /// The hosted designer edited the form's controls or bounds.
    DesignEdited,
    /// Add a control family to the hosted form.
    AddControl(PaletteKind),
    /// Commit the inspector field the user just edited.
    Commit(PropertyField),
    /// Re-anchor the selected control.
    SetAnchor(Anchor),
    /// Resize the form to match the hosted designer pane's current size.
    ResizeFormToPane,
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
