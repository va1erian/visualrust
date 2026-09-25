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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Route {
    pub name: String,
    pub method: HttpMethod,
    pub path: String,
    pub handler: String,
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
