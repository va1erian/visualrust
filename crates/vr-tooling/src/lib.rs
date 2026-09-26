//! Capture, sandbox and automation test support for VisualRust.
//!
//! Two layers:
//!
//! * [`capture_window`] snapshots a live window to a PNG by handle, title or
//!   process id. It uses win32ui's composited (`Windows.Graphics.Capture`)
//!   backend and falls back to `PrintWindow`, so it works without raising,
//!   focusing or unoccluding the target.
//! * [`assert_matches_golden`] compares a capture against a checked-in golden
//!   under `tests/golden/` and fails with a diff percentage.
//!
//! Tests in this crate are quiet and write only to temp directories.

#![deny(unsafe_code)]

mod capture;
mod error;
mod golden;
mod sys;

pub use capture::{WindowSelector, capture_image, capture_window, capture_window_rendered};
pub use error::{Result, ToolingError};
pub use golden::{
    ACCEPT_ENV, Diff, GOLDEN_DIR_ENV, accept_golden, assert_matches_golden,
    assert_selector_matches_golden, diff, golden_dir, golden_path, read_png, write_png,
};
