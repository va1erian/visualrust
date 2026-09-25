//! Typed errors for the capture and golden-image helpers.

use std::path::PathBuf;

use thiserror::Error;

/// A failure while capturing a window or comparing a golden image.
#[derive(Debug, Error)]
pub enum ToolingError {
    /// No visible top-level window matched the selector.
    #[error("no window matched {selector}")]
    WindowNotFound {
        /// The selector that was searched for.
        selector: String,
    },
    /// The handle named by the selector is no longer a live window.
    #[error("window 0x{hwnd:x} is no longer alive")]
    WindowGone {
        /// The raw handle value.
        hwnd: usize,
    },
    /// Both capture backends failed for a resolved window.
    #[error("capture failed for {selector}: {reason}")]
    CaptureFailed {
        /// The selector that resolved to the window.
        selector: String,
        /// Why the composited backend failed.
        reason: String,
    },
    /// The tolerance argument was outside the valid `0.0..=1.0` range.
    #[error("tolerance {tolerance} is out of range; expected 0.0..=1.0")]
    InvalidTolerance {
        /// The rejected value.
        tolerance: f64,
    },
    /// No golden image exists yet under `tests/golden/`.
    #[error(
        "golden `{name}` is missing at {path}; run with the VR_ACCEPT_GOLDEN \
         environment variable set to record it"
    )]
    GoldenMissing {
        /// The golden name.
        name: String,
        /// The path that was expected.
        path: PathBuf,
    },
    /// The captured image and the golden differ in size.
    #[error(
        "golden `{name}` is {golden_width}x{golden_height} but the capture is \
         {actual_width}x{actual_height}; sizes must match"
    )]
    GoldenSize {
        /// The golden name.
        name: String,
        /// Captured width.
        actual_width: u32,
        /// Captured height.
        actual_height: u32,
        /// Golden width.
        golden_width: u32,
        /// Golden height.
        golden_height: u32,
    },
    /// The captured image differs from the golden by more than the tolerance.
    #[error(
        "golden `{name}` differs by {diff_percent:.3}% (tolerance \
         {tolerance_percent:.3}%); the capture was written to {actual}"
    )]
    GoldenDiff {
        /// The golden name.
        name: String,
        /// The observed difference, as a percentage of all pixels.
        diff_percent: f64,
        /// The allowed difference, as a percentage.
        tolerance_percent: f64,
        /// Where the mismatching capture was written for inspection.
        actual: PathBuf,
    },
    /// Reading or writing a file failed.
    #[error("I/O error at {path}: {source}")]
    Io {
        /// The path involved.
        path: PathBuf,
        /// The underlying error.
        #[source]
        source: std::io::Error,
    },
    /// Encoding or decoding a PNG failed.
    #[error("PNG error at {path}: {message}")]
    Png {
        /// The path involved.
        path: PathBuf,
        /// The underlying message.
        message: String,
    },
}

/// The crate's result alias.
pub type Result<T> = std::result::Result<T, ToolingError>;
