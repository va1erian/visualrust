//! The VisualRust IDE shell.
//!
//! This crate builds the IDE's main window as a `xui::App`: a light/dark
//! themed window with a File / Edit / View / Build / Run / Help menu bar, a
//! toolbar, an editor pane and a status bar. The editor pane hosts the real
//! Scintilla control from `vr-scintilla`, showing a Dyon document with syntax
//! highlighting and line numbers; the forms designer and other panes land in
//! the same frame over M3.
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
//! starting palette. `tests/editor_capture.rs` calls [`run_with`] with a
//! `.dyon` file, waits for the container lexer to style it, and captures the
//! pane; it asserts several of the Dyon palette colours are present.

#![forbid(unsafe_code)]

mod app;
mod document;
mod menus;
mod msg;
mod state;
mod toolbar;

pub use app::{IdeApp, TITLE};
pub use msg::Msg;
pub use state::{Effect, IdeState};

use std::path::PathBuf;

use xui::{WindowSpec, dip};

/// Why the IDE could not start.
#[derive(Debug, thiserror::Error)]
pub enum IdeError {
    /// The top-level window could not be created, or the message loop failed.
    #[error("the IDE window failed: {0}")]
    Window(#[from] xui::Error),
}

/// Creates the shell's window and runs its message loop until it closes.
///
/// The first non-flag command-line argument is opened in the editor:
/// `cargo run -p vr-ide -- path/to/file.dyon`. Without one the IDE falls back to
/// the nearest project's manifest entry and then to the bundled sample.
pub fn run() -> Result<(), IdeError> {
    let path = document::path_from_args(std::env::args().skip(1));
    run_with(path)
}

/// Like [`run`], but opens `path` instead of reading the command line, so an
/// integration test can drive the editor pane with a known document.
pub fn run_with(path: Option<PathBuf>) -> Result<(), IdeError> {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    xui::run_app(spec(), move |ui| IdeApp::build(ui, path, cwd))?;
    Ok(())
}

/// The initial window: title, size and starting palette.
fn spec() -> WindowSpec {
    WindowSpec::new(TITLE)
        .size(dip(1100.0), dip(720.0))
        .theme(IdeApp::initial_theme())
}
