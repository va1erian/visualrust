//! Data shape of a `vrproj.toml` manifest.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// A VisualRust project as stored on disk.
///
/// Field order matters for serialization: `toml` emits values before tables,
/// so the scalar-bearing `project` table must stay ahead of the collections.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub project: Project,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub forms: Vec<NamedPath>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub modules: Vec<NamedPath>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub assets: Vec<NamedPath>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extensions: Vec<NamedPath>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub routes: Vec<Route>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub db: Option<DbConfig>,
}

/// Identity and entry point of the project.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Project {
    pub name: String,
    pub version: String,
    #[serde(rename = "type")]
    pub kind: ProjectKind,
    /// Defaulted so a missing entry surfaces as a domain error, not a parser
    /// error; validation then reports it with a stable message.
    #[serde(default)]
    pub entry: PathBuf,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Which runtime an exported project targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProjectKind {
    Desktop,
    Console,
    Web,
}

/// A name -> relative path reference (form, module, asset or extension).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NamedPath {
    pub name: String,
    pub path: PathBuf,
}

/// One HTTP endpoint exposed by a web project.
///
/// The `description`, `params` and `response` fields are API-design metadata
/// added for the API editor (#71). They are `#[serde(default)]` so manifests
/// written before they existed still parse, and omitted when unset so existing
/// files keep their shape.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Route {
    pub name: String,
    pub method: HttpMethod,
    pub path: String,
    pub handler: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub params: Vec<RouteParam>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response: Option<SampleResponse>,
}

/// Where a declared request parameter travels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ParamLocation {
    Path,
    Query,
    Body,
}

impl std::fmt::Display for ParamLocation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            ParamLocation::Path => "path",
            ParamLocation::Query => "query",
            ParamLocation::Body => "body",
        })
    }
}

/// A declared request parameter, independent of the `:name` captures in `path`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RouteParam {
    pub name: String,
    pub location: ParamLocation,
    #[serde(default)]
    pub required: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// An example response, shaped like the Dyon dispatch object from #86.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SampleResponse {
    pub status: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    #[serde(
        default = "default_content_type",
        skip_serializing_if = "is_default_content_type"
    )]
    pub content_type: String,
}

impl Default for SampleResponse {
    fn default() -> Self {
        Self {
            status: 200,
            body: None,
            content_type: default_content_type(),
        }
    }
}

/// The content type the Dyon binding (#86) assumes when a handler omits one.
pub(crate) fn default_content_type() -> String {
    "text/plain; charset=utf-8".to_owned()
}

fn is_default_content_type(value: &str) -> bool {
    value == default_content_type()
}

/// Supported HTTP verbs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
    Head,
    Options,
}

impl std::fmt::Display for HttpMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let verb = match self {
            HttpMethod::Get => "GET",
            HttpMethod::Post => "POST",
            HttpMethod::Put => "PUT",
            HttpMethod::Patch => "PATCH",
            HttpMethod::Delete => "DELETE",
            HttpMethod::Head => "HEAD",
            HttpMethod::Options => "OPTIONS",
        };
        f.write_str(verb)
    }
}

/// The database a project opens at runtime.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DbConfig {
    pub path: PathBuf,
}
