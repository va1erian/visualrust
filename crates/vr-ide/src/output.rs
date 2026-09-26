//! The bottom output/status pane.
//!
//! A read-only multi-line edit: it scrolls, selects and copies like any native
//! control, but the user cannot type into it. The shell writes whole lines with
//! [`OutputPane::set_lines`]; the log itself lives in
//! [`IdeState`](crate::IdeState), so the view stays a dumb terminal.

use xui::prelude::*;

use crate::Msg;

/// A read-only text view of the shell's output log.
pub struct OutputPane {
    edit: Edit<Msg>,
}

impl OutputPane {
    /// Creates the pane as a child of the window behind `ui`.
    pub fn new(ui: &mut Ui<Msg>) -> xui::Result<OutputPane> {
        let edit = Edit::multi_line(ui)?.read_only(true);
        Ok(OutputPane { edit })
    }

    /// Replaces the pane's contents with `lines`, one per row.
    pub fn set_lines(&self, lines: &[String]) {
        // Win32 edits want CRLF between rows; `to_string` is not needed because
        // the edit re-reads its own buffer after the write.
        self.edit.set_text(&lines.join("\r\n"));
    }
}

impl AsControl for OutputPane {
    fn control(&self) -> &Control {
        self.edit.control()
    }
}
