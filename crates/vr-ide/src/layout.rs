//! The shell's layout tree: explorer | central pane, with the output pane
//! stacked under the centre and the property inspector under the explorer.
//!
//! The explorer, output and inspector panes are always present; the central
//! pane is the editor by default and the real form designer when design mode is
//! on. Exactly one of the editor and the designer is visible, so the visible one
//! fills the slot.

use xui::prelude::*;

use crate::Msg;
use crate::designer::DesignPane;
use crate::inspector::PropertyInspector;
use crate::output::OutputPane;

/// The explorer's starting width, in design units.
const EXPLORER_WIDTH: f64 = 220.0;
/// The property inspector's height, in design units, when design mode is on.
const INSPECTOR_HEIGHT: f32 = 430.0;

/// The widgets the shell builds, so a pane that failed to create can be `None`
/// without the builder taking eight arguments.
pub(crate) struct Panes<'a> {
    pub toolbar: &'a Option<Toolbar<Msg>>,
    pub explorer: &'a Option<TreeView<usize, Msg>>,
    pub editor: &'a Option<vr_scintilla::ScintillaHost<Msg>>,
    pub output: &'a Option<OutputPane>,
    pub design: &'a Option<DesignPane>,
    pub inspector: &'a Option<PropertyInspector>,
    pub status: &'a Option<StatusBar<Msg>>,
}

/// Builds the column, skipping a bar or pane that failed to create.
pub(crate) fn build(panes: Panes<'_>, central_height: Dip) -> Layout {
    // The left column stacks the explorer over the inspector; a missing pane
    // degrades to an empty layout that still absorbs its slot, so the remaining
    // panes keep their edges.
    let mut left = Layout::column();
    match panes.explorer {
        Some(tree) => left = left.item(tree.fill(1)),
        None => left = left.item(Layout::column().fill(1)),
    }
    if let Some(pane) = panes.inspector {
        left = left.item(pane.height(dip(INSPECTOR_HEIGHT)));
    }

    // The editor and the design pane share the centre; a hidden item takes no
    // space, so the visible one fills the slot.
    let mut central = Layout::column();
    if let Some(host) = panes.editor {
        central = central.item(host.fill(1));
    }
    if let Some(pane) = panes.design {
        central = central.item(pane.fill(1));
    }
    let bottom = match panes.output {
        Some(pane) => pane.fill(1),
        None => Layout::column().fill(1),
    };

    // `position` is the first pane's extent, so the divider opens near the
    // bottom of the window; the output pane keeps the remainder.
    let centre = split_col![central, bottom].position(central_height);
    // `central` is a `Layout` (a column holding both centre widgets).
    let body = split_row![left, centre].position(dip(EXPLORER_WIDTH as f32));

    let mut layout = Layout::column();
    if let Some(bar) = panes.toolbar {
        layout = layout.item(bar);
    }
    layout = layout.item(body);
    if let Some(bar) = panes.status {
        layout = layout.item(bar);
    }
    layout
}
