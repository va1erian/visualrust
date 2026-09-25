//! Typed failures for endpoint modelling, persistence and OpenAPI mapping.

use thiserror::Error;

use vr_core::manifest::ManifestError;

/// Anything that can go wrong while building, persisting or scaffolding an API.
///
/// Every variant is a value error; no operation in this crate panics on bad
/// input, so the IDE can surface the message and keep the project open.
#[derive(Debug, Error)]
pub enum ApiError {
    /// Loading or saving the manifest the endpoints live in failed.
    #[error(transparent)]
    Manifest(#[from] ManifestError),

    /// The OpenAPI document was not valid JSON.
    #[error("OpenAPI document is not valid JSON: {0}")]
    Json(#[from] serde_json::Error),

    /// The document had no `paths` object.
    #[error("OpenAPI document has no `paths` object")]
    MissingPaths,

    /// A path item was not an object.
    #[error("OpenAPI path `{path}` is not an object")]
    InvalidPathItem { path: String },

    /// An operation was not an object.
    #[error("OpenAPI operation for `{method} {path}` is not an object")]
    InvalidOperation { method: String, path: String },

    /// An endpoint has no handler name to scaffold.
    #[error("endpoint handler name must not be empty")]
    EmptyHandler,

    /// The route path does not begin with `/`, so the router could never match.
    #[error("endpoint path `{path}` must start with '/'")]
    InvalidPath { path: String },

    /// The same `:param` appears twice in one path.
    #[error("endpoint `{name}` declares duplicate path parameter `{param}`")]
    DuplicateParam { name: String, param: String },
}
