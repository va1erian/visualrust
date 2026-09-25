//! The safe model of a Scintilla `SCN_*` notification.
//!
//! Scintilla delivers these as an `SCNotification` through `WM_NOTIFY`. The raw
//! pointer is dereferenced in [`crate::sys`]; this module maps the copied
//! struct into [`Scn`] and owns any string it points at.

use vr_scintilla_sys::ScNotification;
use vr_scintilla_sys::notifications::{
    SCN_AUTOCCOMPLETED, SCN_CALLTIPCLICK, SCN_CHARADDED, SCN_DOUBLECLICK, SCN_FOCUSIN,
    SCN_FOCUSOUT, SCN_MARGINCLICK, SCN_MARGINRIGHTCLICK, SCN_MODIFIED, SCN_MODIFYATTEMPTRO,
    SCN_PAINTED, SCN_SAVEPOINTLEFT, SCN_SAVEPOINTREACHED, SCN_STYLENEEDED, SCN_UPDATEUI, SCN_ZOOM,
};

/// A decoded `SCN_*` notification.
///
/// Variants carry only the fields that are meaningful for their code. Text is
/// copied out of Scintilla's buffer, so a `Scn` may outlive the notification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Scn {
    /// The container lexer must style up to `position`.
    StyleNeeded {
        /// The end position of the region needing styles.
        position: isize,
    },
    /// A character was typed.
    CharAdded {
        /// The typed character.
        ch: char,
    },
    /// The document changed; `text` is the inserted or deleted text when the
    /// modification carried any.
    Modified {
        /// Where the change happened.
        position: isize,
        /// A `SC_MOD_*` bit set describing the change.
        modification_type: i32,
        /// The change's text, if the notification carried it.
        text: Option<String>,
        /// Length of the change in bytes.
        length: isize,
        /// Lines added or removed by the change.
        lines_added: isize,
        /// The originating `SCI_*` message, when known.
        message: i32,
    },
    /// The document reached its save point.
    SavePointReached,
    /// The document moved away from its save point.
    SavePointLeft,
    /// A change was attempted while the document was read-only.
    ModifyAttemptRo,
    /// The user interface needs refreshing.
    UpdateUi {
        /// A `SC_UPDATE_*` bit set describing what changed.
        updated: i32,
    },
    /// A margin was clicked.
    MarginClick {
        /// The document position of the click.
        position: isize,
        /// Modifier keys held during the click.
        modifiers: i32,
        /// The margin number.
        margin: i32,
    },
    /// A margin was right-clicked.
    MarginRightClick {
        /// The document position of the click.
        position: isize,
        /// Modifier keys held during the click.
        modifiers: i32,
        /// The margin number.
        margin: i32,
    },
    /// The control was double-clicked.
    DoubleClick {
        /// The document position of the click.
        position: isize,
        /// The line of the click.
        line: isize,
    },
    /// The control finished painting.
    Painted,
    /// The zoom level changed.
    Zoom,
    /// The control gained focus.
    FocusIn,
    /// The control lost focus.
    FocusOut,
    /// A call tip was clicked.
    CallTipClick {
        /// The click position within the call tip.
        position: isize,
    },
    /// An autocompletion entry was accepted.
    AutoCCompleted {
        /// The document position of the accepted entry.
        position: isize,
        /// The accepted text, if the notification carried it.
        text: Option<String>,
        /// How the entry was completed (`SC_AC_*`).
        list_completion_method: i32,
    },
    /// A notification this wrapper does not model.
    Other {
        /// The raw `SCN_*` / `SCEN_*` code.
        code: u32,
    },
}

impl Scn {
    /// Maps a copied [`ScNotification`] to a [`Scn`]. `text` is the string the
    /// raw notification pointed at, already copied by the caller.
    pub(crate) fn from_notification(notification: &ScNotification, text: Option<String>) -> Scn {
        match notification.nmhdr.code {
            SCN_STYLENEEDED => Scn::StyleNeeded {
                position: notification.position,
            },
            SCN_CHARADDED => Scn::CharAdded {
                ch: char::from_u32(notification.ch as u32).unwrap_or(char::REPLACEMENT_CHARACTER),
            },
            SCN_MODIFIED => Scn::Modified {
                position: notification.position,
                modification_type: notification.modification_type,
                text,
                length: notification.length,
                lines_added: notification.lines_added,
                message: notification.message,
            },
            SCN_SAVEPOINTREACHED => Scn::SavePointReached,
            SCN_SAVEPOINTLEFT => Scn::SavePointLeft,
            SCN_MODIFYATTEMPTRO => Scn::ModifyAttemptRo,
            SCN_UPDATEUI => Scn::UpdateUi {
                updated: notification.updated,
            },
            SCN_MARGINCLICK => Scn::MarginClick {
                position: notification.position,
                modifiers: notification.modifiers,
                margin: notification.margin,
            },
            SCN_MARGINRIGHTCLICK => Scn::MarginRightClick {
                position: notification.position,
                modifiers: notification.modifiers,
                margin: notification.margin,
            },
            SCN_DOUBLECLICK => Scn::DoubleClick {
                position: notification.position,
                line: notification.line,
            },
            SCN_PAINTED => Scn::Painted,
            SCN_ZOOM => Scn::Zoom,
            SCN_FOCUSIN => Scn::FocusIn,
            SCN_FOCUSOUT => Scn::FocusOut,
            SCN_CALLTIPCLICK => Scn::CallTipClick {
                position: notification.position,
            },
            SCN_AUTOCCOMPLETED => Scn::AutoCCompleted {
                position: notification.position,
                text,
                list_completion_method: notification.list_completion_method,
            },
            code => Scn::Other { code },
        }
    }
}
