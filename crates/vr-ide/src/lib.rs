//! The VisualRust IDE shell.
//!
//! This crate builds the IDE's main window as a `xui::App`: a light/dark
//! themed window with a File / Edit / View / Build / Run / Help menu bar, a
//! toolbar and a status bar. It is the frame the editor, forms designer and
//! other panes land in over M1–M3; for now the centre is a placeholder.
//!
//! ## View model
//!
//! The shell follows a small unidirectional loop, documented in full in the
//! [`state`] module:
//!
//! ```text
//!   chrome (menus / toolbar / accelerators / timer)
//!         |  Msg
//!         v
//!   IdeState::apply     pure reducer, no xui types
//!         |  Effect
//!         v
//!   IdeApp::update      the only code that touches the live window
//! ```
//!
//! Widgets raise a [`Msg`] rather than mutating the window; [`IdeState`] folds
//! the message into state and returns an [`Effect`] that `IdeApp::update`
//! applies (switch theme, relayout, set status text, quit). Because the
//! reducer is pure it can be tested with no desktop, and because xui
//! delivers one queued message at a time `update` is never re-entered.
//!
//! ## Verifying it
//!
//! `xui_DEMO_AUTOCLOSE_MS` makes the window quit itself, which the
//! `tests/smoke.rs` integration test uses: it runs the binary, captures the
//! live window with `vr-tooling`, and reports the PNG path and size (or skips
//! when the session has no desktop). `xui_DEMO_THEME=light|dark` picks the
//! starting palette.

#![forbid(unsafe_code)]

mod app;
mod menus;
mod msg;
mod state;
mod toolbar;

pub use app::{IdeApp, TITLE};
pub use msg::Msg;
pub use state::{Effect, IdeState};

use xui::{WindowSpec, dip};

/// Why the IDE could not start.
#[derive(Debug, thiserror::Error)]
pub enum IdeError {
    /// The top-level window could not be created, or the message loop failed.
    #[error("the IDE window failed: {0}")]
    Window(#[from] xui::Error),
}

/// Creates the shell's window and runs its message loop until it closes.
pub fn run() -> Result<(), IdeError> {
    xui::run_app(spec(), IdeApp::build)?;
    Ok(())
}

/// The initial window: title, size and starting palette.
fn spec() -> WindowSpec {
    WindowSpec::new(TITLE)
        .size(dip(1100.0), dip(720.0))
        .theme(IdeApp::initial_theme())
}
