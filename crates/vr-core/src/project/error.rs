//! Typed failures for project filesystem operations.

use std::io;
use std::path::PathBuf;

use crate::manifest::ManifestError;

/// Anything that can go wrong creating, opening or editing a project.
#[derive(Debug, thiserror::Error)]
pub enum ProjectError {
    #[error(transparent)]
    Manifest(#[from] ManifestError),

    #[error("project directory `{}` already contains files", .path.display())]
    DirectoryNotEmpty { path: PathBuf },

    #[error("project root `{}` is a file, not a directory", .path.display())]
    RootIsFile { path: PathBuf },

    #[error("{section} `{name}` already exists with a different path")]
    DuplicateEntry { section: &'static str, name: String },

    #[error("filesystem operation on `{}` failed: {source}", .path.display())]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
}

impl ProjectError {
    pub(crate) fn io(path: impl Into<PathBuf>, source: io::Error) -> Self {
        Self::Io {
            path: path.into(),
            source,
        }
    }
}
