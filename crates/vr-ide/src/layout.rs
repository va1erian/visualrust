//! The shell's layout tree: explorer | central pane, with the output pane
//! stacked under the centre. The toolbar and status bar bracket the whole
//! column, as before.
//!
//! The explorer and the output pane are always present; the central pane is the
//! editor by default and the designer surface when design mode is on. The
//! designer is not a layout node — see [`crate::designer`] — so it is not built
//! here; design mode simply hides the editor, and the surface's own nodes draw
//! over the central area.

use xui::prelude::*;

use crate::Msg;
use crate::designer::DesignPane;
use crate::output::OutputPane;

/// The explorer's starting width, in design units.
const EXPLORER_WIDTH: f64 = 220.0;

/// Builds the column, skipping a bar that failed to create.
pub(crate) fn build(
    toolbar: &Option<Toolbar<Msg>>,
    explorer: &Option<TreeView<usize, Msg>>,
    editor: &Option<vr_scintilla::ScintillaHost<Msg>>,
    output: &Option<OutputPane>,
    design: &Option<DesignPane>,
    status: &Option<StatusBar<Msg>>,
    central_height: Dip,
) -> Layout {
    // A missing pane degrades to an empty layout that still absorbs its slot,
    // so the remaining panes keep their edges.
    let left = match explorer {
        Some(tree) => tree.fill(1),
        None => Layout::column().fill(1),
    };
    // The editor and the design pane share the centre; exactly one is visible,
    // and a hidden item takes no space, so the visible one fills the slot.
    let mut central = Layout::column();
    if let Some(host) = editor {
        central = central.item(host.fill(1));
    }
    if let Some(pane) = design {
        central = central.item(pane.fill(1));
    }
    let bottom = match output {
        Some(pane) => pane.fill(1),
        None => Layout::column().fill(1),
    };

    // `position` is the first pane's extent, so the divider opens near the
    // bottom of the window; the output pane keeps the remainder.
    let centre = split_col![central, bottom].position(central_height);
    // `central` is a `Layout` (a column holding both centre widgets).
    let body = split_row![left, centre].position(dip(EXPLORER_WIDTH as f32));

    let mut layout = Layout::column();
    if let Some(bar) = toolbar {
        layout = layout.item(bar);
    }
    layout = layout.item(body);
    if let Some(bar) = status {
        layout = layout.item(bar);
    }
    layout
}
