//! The typed project payload: a manifest plus named, categorized blobs.

use std::path::Path;

use vr_core::manifest::Manifest;

use crate::error::PackError;

/// What a blob is, so the runtime can assemble the app without guessing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryKind {
    /// A Dyon source file (the entry point or a module).
    Source,
    /// A precompiled `.vrform` form definition.
    Form,
    /// A non-code asset (image, text, data file).
    Asset,
    /// The project's SQLite database.
    Database,
    /// A native extension DLL listed in the manifest.
    Extension,
}

impl EntryKind {
    /// Stable on-disk tag; never renumber an existing variant.
    pub(crate) fn as_u8(self) -> u8 {
        match self {
            EntryKind::Source => 0,
            EntryKind::Form => 1,
            EntryKind::Asset => 2,
            EntryKind::Database => 3,
            EntryKind::Extension => 4,
        }
    }

    pub(crate) fn from_u8(tag: u8) -> Result<Self, PackError> {
        match tag {
            0 => Ok(EntryKind::Source),
            1 => Ok(EntryKind::Form),
            2 => Ok(EntryKind::Asset),
            3 => Ok(EntryKind::Database),
            4 => Ok(EntryKind::Extension),
            other => Err(PackError::Malformed(format!("unknown entry kind {other}"))),
        }
    }
}

/// One named blob inside a bundle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BundleEntry {
    pub kind: EntryKind,
    /// Manifest name for this blob (module/form/asset/extension name, or
    /// `"entry"` / `"database"` for the singletons).
    pub name: String,
    pub data: Vec<u8>,
}

/// A complete project payload: the validated manifest and its blobs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bundle {
    pub manifest: Manifest,
    pub(crate) entries: Vec<BundleEntry>,
}

impl Bundle {
    /// Wraps a manifest with no blobs yet.
    pub fn new(manifest: Manifest) -> Self {
        Self {
            manifest,
            entries: Vec::new(),
        }
    }

    /// Appends a Dyon source keyed by its manifest name.
    pub fn add_source(&mut self, name: impl Into<String>, data: impl Into<Vec<u8>>) {
        self.push(EntryKind::Source, name.into(), data.into());
    }

    /// Appends a precompiled form.
    pub fn add_form(&mut self, name: impl Into<String>, data: impl Into<Vec<u8>>) {
        self.push(EntryKind::Form, name.into(), data.into());
    }

    /// Appends a static asset.
    pub fn add_asset(&mut self, name: impl Into<String>, data: impl Into<Vec<u8>>) {
        self.push(EntryKind::Asset, name.into(), data.into());
    }

    /// Appends a native extension DLL.
    pub fn add_extension(&mut self, name: impl Into<String>, data: impl Into<Vec<u8>>) {
        self.push(EntryKind::Extension, name.into(), data.into());
    }

    /// Stores the project database, replacing any previous one: a bundle has at
    /// most one.
    pub fn set_database(&mut self, data: impl Into<Vec<u8>>) {
        self.entries.retain(|e| e.kind != EntryKind::Database);
        self.push(EntryKind::Database, "database".to_owned(), data.into());
    }

    fn push(&mut self, kind: EntryKind, name: String, data: Vec<u8>) {
        self.entries.push(BundleEntry { kind, name, data });
    }

    pub fn entries(&self) -> &[BundleEntry] {
        &self.entries
    }

    /// Finds a blob by kind and name.
    pub fn entry(&self, kind: EntryKind, name: &str) -> Option<&BundleEntry> {
        self.entries
            .iter()
            .find(|e| e.kind == kind && e.name == name)
    }

    /// The stored SQLite database, if the project has one.
    pub fn database(&self) -> Option<&[u8]> {
        self.entry(EntryKind::Database, "database")
            .map(|e| e.data.as_slice())
    }

    /// Reads every file a manifest references from `root` into a ready bundle.
    ///
    /// This is the packaging half of `from_project`: it does no validation of
    /// its own because [`Manifest`] was already validated when loaded, so any
    /// failure here is a missing or unreadable file.
    pub fn from_project(root: &Path, manifest: Manifest) -> Result<Self, PackError> {
        let mut bundle = Bundle::new(manifest.clone());

        bundle.add_source("entry", read_rel(root, &manifest.project.entry)?);
        for module in &manifest.modules {
            bundle.add_source(module.name.clone(), read_rel(root, &module.path)?);
        }
        for form in &manifest.forms {
            bundle.add_form(form.name.clone(), read_rel(root, &form.path)?);
        }
        for asset in &manifest.assets {
            bundle.add_asset(asset.name.clone(), read_rel(root, &asset.path)?);
        }
        for extension in &manifest.extensions {
            bundle.add_extension(extension.name.clone(), read_rel(root, &extension.path)?);
        }
        if let Some(db) = &manifest.db {
            bundle.set_database(read_rel(root, &db.path)?);
        }

        Ok(bundle)
    }
}

fn read_rel(root: &Path, rel: &Path) -> Result<Vec<u8>, PackError> {
    let path = root.join(rel);
    std::fs::read(&path).map_err(|source| PackError::ReadFile { path, source })
}
