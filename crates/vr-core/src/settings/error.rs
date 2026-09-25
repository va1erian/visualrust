//! Typed failures for loading, saving and validating user settings.

use std::io;
use std::path::PathBuf;

/// Anything that can go wrong reading or writing the settings store.
#[derive(Debug, thiserror::Error)]
pub enum SettingsError {
    #[error("APPDATA is not set; cannot locate the VisualRust settings directory")]
    MissingAppData,

    #[error("could not read settings `{}`: {source}", .path.display())]
    Read {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("could not write settings `{}`: {source}", .path.display())]
    Write {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("settings are not valid TOML: {0}")]
    Parse(#[from] toml::de::Error),

    #[error("settings could not be serialized: {0}")]
    Serialize(#[from] toml::ser::Error),

    #[error("editor font size {0} is out of the range 1..=200")]
    FontSizeOutOfRange(u32),

    #[error("editor tab width {0} is out of the range 1..=16")]
    TabWidthOutOfRange(u32),

    #[error("editor font family must not be empty")]
    EmptyFontFamily,

    #[error("recent project path must not be empty")]
    EmptyRecentPath,
}

impl SettingsError {
    pub(crate) fn write(path: impl Into<PathBuf>, source: io::Error) -> Self {
        Self::Write {
            path: path.into(),
            source,
        }
    }
}
