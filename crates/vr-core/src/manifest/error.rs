//! Typed failures for loading, writing and validating a manifest.

use std::io;
use std::path::PathBuf;

use super::model::HttpMethod;

/// Anything that can go wrong turning bytes into a valid manifest or back.
#[derive(Debug, thiserror::Error)]
pub enum ManifestError {
    #[error("could not read manifest `{}`: {source}", .path.display())]
    Read {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("could not write manifest `{}`: {source}", .path.display())]
    Write {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("manifest is not valid TOML: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("manifest could not be serialized: {0}")]
    Serialize(#[from] toml::ser::Error),
    #[error(transparent)]
    Invalid(#[from] ValidationError),
}

/// A manifest that parsed but describes an unusable project.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ValidationError {
    #[error("{field} must not be empty")]
    EmptyName { field: &'static str },

    #[error("{field} `{name}` is not a valid name; use letters, digits, '-' or '_'")]
    InvalidName { field: &'static str, name: String },

    #[error("project.version must not be empty")]
    EmptyVersion,

    #[error("manifest is missing the project entry; set `entry` under [project]")]
    MissingEntry,

    #[error("project entry `{}` must be a relative path inside the project", .0.display())]
    EntryNotRelative(PathBuf),

    #[error(
        "{section} `{name}` points at `{}`, which must be a relative path inside the project",
        .path.display()
    )]
    UnsafePath {
        section: &'static str,
        name: String,
        path: PathBuf,
    },

    #[error("duplicate {section} name `{name}`")]
    DuplicateName { section: &'static str, name: String },

    #[error("routes are only valid for web projects")]
    RoutesRequireWeb,

    #[error("console projects cannot declare forms")]
    FormsNotAllowedForConsole,

    #[error("route handler must not be empty")]
    EmptyRouteHandler,

    #[error("route path `{path}` must start with '/'")]
    InvalidRoutePath { path: String },

    #[error("duplicate route for {method} {path}")]
    DuplicateRoute { method: HttpMethod, path: String },

    #[error("database path must not be empty")]
    EmptyDbPath,
}
