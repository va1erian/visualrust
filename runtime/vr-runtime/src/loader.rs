//! Locate the bundle appended to an executable and lay its blobs out on disk.
//!
//! Blobs are extracted to a scratch tree rather than kept in memory so the Dyon
//! file loader and the extension DLL loader see real paths, exactly as they
//! would for an unpackaged project. The tree is removed with [`Extracted`].

use std::path::{Component, Path, PathBuf};

use vr_core::manifest::Manifest;
use vr_pack::{Bundle, EntryKind, PackError};

use crate::error::RuntimeError;
use crate::tempdir::TempDir;

/// One blob after extraction: its bundle identity and where it landed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractedEntry {
    pub kind: EntryKind,
    pub name: String,
    pub path: PathBuf,
    pub len: usize,
}

/// A decoded bundle whose blobs are written under a scratch root that is
/// removed when this value drops.
pub struct Extracted {
    guard: TempDir,
    manifest: Manifest,
    entry: PathBuf,
    entries: Vec<ExtractedEntry>,
}

impl Extracted {
    /// The scratch root holding every extracted blob.
    pub fn root(&self) -> &Path {
        self.guard.path()
    }

    /// The manifest's entry source, now a real file.
    pub fn entry_path(&self) -> &Path {
        &self.entry
    }

    pub fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    pub fn entries(&self) -> &[ExtractedEntry] {
        &self.entries
    }

    /// The extracted path of a named blob, if the bundle carried it.
    pub fn path_for(&self, kind: EntryKind, name: &str) -> Option<&Path> {
        self.entries
            .iter()
            .find(|e| e.kind == kind && e.name == name)
            .map(|e| e.path.as_path())
    }
}

/// Reads the bundle footer from `path` and extracts it to a fresh scratch tree.
///
/// A file with no footer is [`RuntimeError::NoBundle`], never a panic, so the
/// stub can distinguish an unpackaged executable from a corrupt payload.
pub fn load_from_file(path: impl AsRef<Path>) -> Result<Extracted, RuntimeError> {
    let path = path.as_ref();
    let bundle = match Bundle::read_from_file(path) {
        Ok(bundle) => bundle,
        Err(PackError::MissingFooter | PackError::TooShort) => {
            return Err(RuntimeError::NoBundle {
                path: path.to_path_buf(),
            });
        }
        Err(error) => return Err(RuntimeError::Pack(error)),
    };
    extract(bundle)
}

fn extract(bundle: Bundle) -> Result<Extracted, RuntimeError> {
    let manifest = bundle.manifest.clone();
    let guard = TempDir::create("app")?;
    let mut entries = Vec::with_capacity(bundle.entries().len());

    for blob in bundle.entries() {
        let rel = target_path(&manifest, blob.kind, &blob.name).ok_or_else(|| {
            RuntimeError::UnmappedEntry {
                kind: kind_name(blob.kind),
                name: blob.name.clone(),
            }
        })?;
        let dest = safe_join(guard.path(), &rel)?;
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent).map_err(|source| RuntimeError::Extract {
                name: blob.name.clone(),
                path: parent.to_path_buf(),
                source,
            })?;
        }
        std::fs::write(&dest, &blob.data).map_err(|source| RuntimeError::Extract {
            name: blob.name.clone(),
            path: dest.clone(),
            source,
        })?;
        entries.push(ExtractedEntry {
            kind: blob.kind,
            name: blob.name.clone(),
            path: dest,
            len: blob.data.len(),
        });
    }

    let entry = safe_join(guard.path(), &manifest.project.entry)?;
    Ok(Extracted {
        guard,
        manifest,
        entry,
        entries,
    })
}

/// Maps a bundled blob to the relative path the manifest says it lives at.
///
/// The entry source is the singleton named `"entry"`; modules and the other
/// kinds are keyed by their manifest name.
fn target_path(manifest: &Manifest, kind: EntryKind, name: &str) -> Option<PathBuf> {
    match kind {
        EntryKind::Source if name == "entry" => Some(manifest.project.entry.clone()),
        EntryKind::Source => named_path(&manifest.modules, name),
        EntryKind::Form => named_path(&manifest.forms, name),
        EntryKind::Asset => named_path(&manifest.assets, name),
        EntryKind::Extension => named_path(&manifest.extensions, name),
        EntryKind::Database => manifest.db.as_ref().map(|db| db.path.clone()),
    }
}

fn named_path(items: &[vr_core::manifest::NamedPath], name: &str) -> Option<PathBuf> {
    items
        .iter()
        .find(|item| item.name == name)
        .map(|item| item.path.clone())
}

/// Joins a validated relative path onto `root`, refusing anything that would
/// escape it. Manifest validation already enforces this, but the bundle crosses
/// a trust boundary, so the loader re-checks before writing.
fn safe_join(root: &Path, rel: &Path) -> Result<PathBuf, RuntimeError> {
    let safe = rel
        .components()
        .all(|c| matches!(c, Component::Normal(_) | Component::CurDir));
    if !safe {
        return Err(RuntimeError::UnsafePath(rel.to_path_buf()));
    }
    Ok(root.join(rel))
}

fn kind_name(kind: EntryKind) -> &'static str {
    match kind {
        EntryKind::Source => "source",
        EntryKind::Form => "form",
        EntryKind::Asset => "asset",
        EntryKind::Database => "database",
        EntryKind::Extension => "extension",
    }
}
