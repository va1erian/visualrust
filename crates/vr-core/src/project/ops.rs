//! Add/remove operations that keep the manifest and the project directory in
//! sync.
//!
//! Each mutating call validates before writing and rolls back both the
//! in-memory manifest and any file it created when a later step fails.

use std::path::PathBuf;

use crate::fsutil::CreatedPaths;
use crate::manifest::{Manifest, NamedPath};

use super::{ASSETS_DIR, FORMS_DIR, Project, ProjectError, SRC_DIR};

/// The manifest collections an operation can target.
#[derive(Debug, Clone, Copy)]
enum Section {
    Modules,
    Forms,
    Assets,
}

impl Section {
    fn label(self) -> &'static str {
        match self {
            Section::Modules => "module",
            Section::Forms => "form",
            Section::Assets => "file",
        }
    }

    fn list_mut(self, manifest: &mut Manifest) -> &mut Vec<NamedPath> {
        match self {
            Section::Modules => &mut manifest.modules,
            Section::Forms => &mut manifest.forms,
            Section::Assets => &mut manifest.assets,
        }
    }
}

impl Project {
    /// Adds a Dyon source module under `src/` and creates its file.
    ///
    /// Calling twice with the same name and path is a no-op.
    pub fn add_module(&mut self, name: &str, path: impl Into<PathBuf>) -> Result<(), ProjectError> {
        let path = path.into();
        debug_assert!(
            path.starts_with(SRC_DIR),
            "module {name} should live under {SRC_DIR}/"
        );
        self.add_named(Section::Modules, name, path)
    }

    /// Adds a `.vrform` under `forms/` and creates its file.
    ///
    /// Calling twice with the same name and path is a no-op.
    pub fn add_form(&mut self, name: &str, path: impl Into<PathBuf>) -> Result<(), ProjectError> {
        let path = path.into();
        debug_assert!(
            path.starts_with(FORMS_DIR),
            "form {name} should live under {FORMS_DIR}/"
        );
        self.add_named(Section::Forms, name, path)
    }

    /// Adds a resource under `assets/` and creates its file.
    ///
    /// Calling twice with the same name and path is a no-op.
    pub fn add_file(&mut self, name: &str, path: impl Into<PathBuf>) -> Result<(), ProjectError> {
        let path = path.into();
        debug_assert!(
            path.starts_with(ASSETS_DIR),
            "asset {name} should live under {ASSETS_DIR}/"
        );
        self.add_named(Section::Assets, name, path)
    }

    /// Removes the named module, form or asset and deletes its file.
    ///
    /// Idempotent: removing an unknown name succeeds without touching the disk.
    pub fn remove(&mut self, name: &str) -> Result<(), ProjectError> {
        let previous = self.manifest.clone();

        let mut removed: Option<NamedPath> = None;
        for section in [Section::Modules, Section::Forms, Section::Assets] {
            let entries = section.list_mut(&mut self.manifest);
            if let Some(index) = entries.iter().position(|entry| entry.name == name) {
                removed = Some(entries.remove(index));
                break;
            }
        }
        let Some(entry) = removed else {
            return Ok(());
        };

        if let Err(err) = self.manifest.validate() {
            self.manifest = previous;
            return Err(ProjectError::Manifest(err.into()));
        }

        let abs = self.root().join(&entry.path);
        let snapshot = std::fs::read(&abs).ok();
        if let Err(source) = std::fs::remove_file(&abs)
            && source.kind() != std::io::ErrorKind::NotFound
        {
            self.manifest = previous;
            return Err(ProjectError::io(abs, source));
        }

        if let Err(err) = self.persist() {
            // Restore the file we just deleted so the project is unchanged.
            if let Some(bytes) = snapshot {
                let _ = std::fs::write(&abs, bytes);
            }
            self.manifest = previous;
            return Err(err);
        }
        Ok(())
    }

    fn add_named(
        &mut self,
        section: Section,
        name: &str,
        path: PathBuf,
    ) -> Result<(), ProjectError> {
        let previous = self.manifest.clone();
        {
            let entries = section.list_mut(&mut self.manifest);
            if let Some(existing) = entries.iter().find(|entry| entry.name == name) {
                if existing.path == path {
                    return Ok(());
                }
                return Err(ProjectError::DuplicateEntry {
                    section: section.label(),
                    name: name.to_owned(),
                });
            }
            entries.push(NamedPath {
                name: name.to_owned(),
                path: path.clone(),
            });
        }

        if let Err(err) = self.manifest.validate() {
            self.manifest = previous;
            return Err(ProjectError::Manifest(err.into()));
        }

        let mut created = CreatedPaths::default();
        let abs = self.root().join(&path);
        if let Err(source) = created.ensure_file(&abs, b"") {
            self.manifest = previous;
            created.rollback();
            return Err(ProjectError::io(abs, source));
        }

        if let Err(err) = self.persist() {
            self.manifest = previous;
            created.rollback();
            return Err(err);
        }
        Ok(())
    }
}
