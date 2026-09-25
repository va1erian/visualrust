//! A filesystem-backed project: a `vrproj.toml` plus its `src/`, `forms/` and
//! `assets/` layout.
//!
//! [`Project`] is the IDE's handle on a project directory; the serialized
//! manifest lives in [`crate::manifest`]. Here we own the directory layout and
//! the operations that keep the manifest and the files in step.

mod error;
mod ops;

#[cfg(test)]
mod tests;

pub use error::ProjectError;

use std::path::{Path, PathBuf};

use crate::fsutil::{self, CreatedPaths};
use crate::manifest::{Manifest, ProjectKind};

/// The manifest filename every project directory carries.
pub const MANIFEST_FILE: &str = "vrproj.toml";
/// Default home for Dyon source files.
pub const SRC_DIR: &str = "src";
/// Default home for `.vrform` files.
pub const FORMS_DIR: &str = "forms";
/// Default home for non-code resources.
pub const ASSETS_DIR: &str = "assets";

/// Options for [`Project::create`], with sensible defaults for everything but
/// the name and kind.
#[derive(Debug, Clone)]
pub struct ProjectOptions {
    pub name: String,
    pub kind: ProjectKind,
    pub version: String,
    pub entry: PathBuf,
    pub description: Option<String>,
}

impl ProjectOptions {
    pub fn new(name: impl Into<String>, kind: ProjectKind) -> Self {
        Self {
            name: name.into(),
            kind,
            version: "0.1.0".to_owned(),
            entry: PathBuf::from(SRC_DIR).join("main.dyon"),
            description: None,
        }
    }

    #[must_use]
    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = version.into();
        self
    }

    #[must_use]
    pub fn with_entry(mut self, entry: impl Into<PathBuf>) -> Self {
        self.entry = entry.into();
        self
    }

    #[must_use]
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
}

/// An open project: its root directory and validated manifest.
#[derive(Debug, Clone)]
pub struct Project {
    root: PathBuf,
    manifest: Manifest,
}

impl Project {
    /// Creates a project directory and its initial manifest.
    ///
    /// Idempotent: if `root` already holds a manifest, the existing project is
    /// opened instead of clobbered. On failure every file and directory this
    /// call created is removed, so no half-built project is left behind.
    pub fn create(root: impl Into<PathBuf>, options: ProjectOptions) -> Result<Self, ProjectError> {
        let root = root.into();
        if root.join(MANIFEST_FILE).is_file() {
            return Self::open(root);
        }
        ensure_empty_or_absent(&root)?;

        // Validate the manifest in memory before touching the disk so an
        // invalid name never leaves directories behind.
        let manifest = Manifest {
            project: crate::manifest::Project {
                name: options.name,
                version: options.version,
                kind: options.kind,
                entry: options.entry,
                description: options.description,
            },
            forms: Vec::new(),
            modules: Vec::new(),
            assets: Vec::new(),
            extensions: Vec::new(),
            routes: Vec::new(),
            db: None,
        };
        let text = manifest.to_toml()?;

        let mut created = CreatedPaths::default();
        let result = (|| -> std::io::Result<()> {
            created.ensure_dir(&root)?;
            for dir in [SRC_DIR, FORMS_DIR, ASSETS_DIR] {
                created.ensure_dir(&root.join(dir))?;
            }
            created.ensure_file(&root.join(&manifest.project.entry), b"")?;
            created.ensure_file(&root.join(MANIFEST_FILE), text.as_bytes())
        })();

        if let Err(source) = result {
            created.rollback();
            return Err(ProjectError::io(root, source));
        }
        Ok(Self { root, manifest })
    }

    /// Opens an existing project directory and validates its manifest.
    ///
    /// Missing standard directories are recreated so later `add_*` calls have
    /// somewhere to write; this keeps `open` safe to call repeatedly.
    pub fn open(root: impl Into<PathBuf>) -> Result<Self, ProjectError> {
        let root = root.into();
        let manifest = Manifest::load(root.join(MANIFEST_FILE))?;
        for dir in [SRC_DIR, FORMS_DIR, ASSETS_DIR] {
            let path = root.join(dir);
            std::fs::create_dir_all(&path).map_err(|source| ProjectError::io(path, source))?;
        }
        Ok(Self { root, manifest })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    pub fn name(&self) -> &str {
        &self.manifest.project.name
    }

    pub fn kind(&self) -> ProjectKind {
        self.manifest.project.kind
    }

    pub fn manifest_path(&self) -> PathBuf {
        self.root.join(MANIFEST_FILE)
    }

    pub fn entry_path(&self) -> PathBuf {
        self.root.join(&self.manifest.project.entry)
    }

    /// Persists the current manifest atomically. Callers are responsible for
    /// having validated it first.
    pub(crate) fn persist(&self) -> Result<(), ProjectError> {
        let text = self.manifest.to_toml()?;
        let path = self.manifest_path();
        fsutil::write_atomic(&path, text.as_bytes())
            .map_err(|source| ProjectError::io(path, source))
    }
}

/// Refuses to build a project on top of unrelated files.
fn ensure_empty_or_absent(root: &Path) -> Result<(), ProjectError> {
    if root.is_file() {
        return Err(ProjectError::RootIsFile {
            path: root.to_path_buf(),
        });
    }
    if root.is_dir() {
        let mut entries =
            std::fs::read_dir(root).map_err(|source| ProjectError::io(root, source))?;
        if entries.next().is_some() {
            return Err(ProjectError::DirectoryNotEmpty {
                path: root.to_path_buf(),
            });
        }
    }
    Ok(())
}
