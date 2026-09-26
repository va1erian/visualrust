//! The styling/theme `SCI_*` sends on a [`Control`](super::Control).
//!
//! These are split from `window.rs` so the creation and teardown code stays
//! readable; they only use the scalar-parameter `send` there, so no raw pointer
//! or additional `unsafe` is needed.

use crate::messages::{
    SCI_GETENDSTYLED, SCI_LINEFROMPOSITION, SCI_POSITIONFROMLINE, SCI_SETCARETFORE,
    SCI_SETMARGINBACKN, SCI_SETSELBACK, SCI_SETSELFORE, SCI_STYLECLEARALL,
};

use super::Control;

impl Control {
    /// Copies `STYLE_DEFAULT` over every style, `SCI_STYLECLEARALL`. This is
    /// what gives the whole viewport (the area beyond the document, the
    /// margins) the theme background rather than Scintilla's default white.
    pub(crate) fn style_clear_all(&self) {
        self.send(SCI_STYLECLEARALL, 0, 0);
    }

    /// The byte position up to which the container lexer has styled,
    /// `SCI_GETENDSTYLED`.
    pub(crate) fn end_styled(&self) -> usize {
        usize::try_from(self.send(SCI_GETENDSTYLED, 0, 0)).unwrap_or(0)
    }

    /// The line containing `position`, `SCI_LINEFROMPOSITION`.
    pub(crate) fn line_from_position(&self, position: i32) -> i32 {
        i32::try_from(self.send(SCI_LINEFROMPOSITION, position as usize, 0)).unwrap_or(0)
    }

    /// The first byte of `line`, `SCI_POSITIONFROMLINE`.
    pub(crate) fn position_from_line(&self, line: i32) -> i32 {
        i32::try_from(self.send(SCI_POSITIONFROMLINE, line as usize, 0)).unwrap_or(0)
    }

    /// Sets a margin's background colour, `SCI_SETMARGINBACKN`.
    pub(crate) fn margin_back(&self, margin: i32, color: i32) {
        self.send(SCI_SETMARGINBACKN, margin as usize, color as isize);
    }

    /// Sets the selection foreground colour, `SCI_SETSELFORE`.
    pub(crate) fn selection_fore(&self, color: i32) {
        self.send(SCI_SETSELFORE, 1, color as isize);
    }

    /// Sets the selection background colour, `SCI_SETSELBACK`.
    pub(crate) fn selection_back(&self, color: i32) {
        self.send(SCI_SETSELBACK, 1, color as isize);
    }

    /// Sets the caret colour, `SCI_SETCARETFORE`.
    pub(crate) fn caret_fore(&self, color: i32) {
        self.send(SCI_SETCARETFORE, color as usize, 0);
    }
}
