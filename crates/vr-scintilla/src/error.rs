//! Typed errors for control creation and text decoding.

use thiserror::Error;

/// A failure while creating a control or decoding its text.
#[derive(Debug, Error)]
pub enum Error {
    /// The Scintilla window class could not be registered, or the control's
    /// host or control window could not be created. This is expected on a
    /// session without an interactive desktop.
    #[error("could not create the Scintilla control window")]
    CreateControl,
    /// The parent window (a win32ui `Custom`) could not be subclassed, so
    /// `SCN_*` notifications would be lost. This is expected on a session
    /// without an interactive desktop.
    #[error("could not subclass the Scintilla parent window")]
    SubclassParent,
    /// The control returned bytes that are not valid UTF-8. The wrapper sets
    /// the UTF-8 code page, so this indicates a corrupted document.
    #[error("the control returned text that is not valid UTF-8")]
    TextEncoding,
}

/// The crate's result alias.
pub type Result<T> = std::result::Result<T, Error>;
