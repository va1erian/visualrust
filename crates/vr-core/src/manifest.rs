//! `vrproj.toml`: the on-disk description of a VisualRust project.
//!
//! The data shape lives in [`model`], the rules in [`validate`]; this module
//! owns the load/save boundary so callers always get a validated [`Manifest`].

mod error;
mod model;
mod validate;

#[cfg(test)]
mod tests;

pub use error::{ManifestError, ValidationError};
pub use model::{DbConfig, HttpMethod, Manifest, NamedPath, Project, ProjectKind, Route};

use std::path::Path;

impl Manifest {
    /// Parses and validates manifest text.
    pub fn from_toml(input: &str) -> Result<Self, ManifestError> {
        let manifest: Self = toml::from_str(input)?;
        manifest.validate()?;
        Ok(manifest)
    }

    /// Validates then serializes to pretty TOML.
    pub fn to_toml(&self) -> Result<String, ManifestError> {
        self.validate()?;
        Ok(toml::to_string_pretty(self)?)
    }

    /// Reads, parses and validates a `vrproj.toml` from disk.
    pub fn load(path: impl AsRef<Path>) -> Result<Self, ManifestError> {
        let path = path.as_ref();
        let text = std::fs::read_to_string(path).map_err(|source| ManifestError::Read {
            path: path.to_path_buf(),
            source,
        })?;
        Self::from_toml(&text)
    }

    /// Validates then writes the manifest to disk.
    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), ManifestError> {
        let path = path.as_ref();
        let text = self.to_toml()?;
        std::fs::write(path, text).map_err(|source| ManifestError::Write {
            path: path.to_path_buf(),
            source,
        })
    }

    /// Checks the manifest against every structural rule.
    pub fn validate(&self) -> Result<(), ValidationError> {
        validate::validate(self)
    }
}
