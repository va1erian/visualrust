//! Directory commands, split from the file-handle code in the parent module.

use std::path::Path;

use crate::error::FileError;

use super::io_error;

/// The kind of a directory entry, mapped to a PureBasic-style flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryKind {
    /// A regular file.
    File,
    /// A directory.
    Directory,
    /// Anything else: a device, socket or symbolic link.
    Other,
}

impl EntryKind {
    /// The numeric flag exposed to Dyon.
    ///
    /// The values follow PureBasic's directory-entry constants so the numbers
    /// are stable and self-documenting at the call site.
    pub fn flag(self) -> f64 {
        match self {
            Self::File => 0.0,
            Self::Directory => 1.0,
            Self::Other => 2.0,
        }
    }
}

/// One entry of a directory listing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirEntry {
    /// The entry's final path component.
    pub name: String,
    /// What kind of entry it is.
    pub kind: EntryKind,
}

/// Creates a single directory at `path` (PureBasic `CreateDirectory`).
///
/// The parent must already exist and `path` must not: this matches
/// `std::fs::create_dir` and makes a duplicate call a typed `AlreadyExists`
/// error rather than a silent success.
pub fn create_directory(path: &str) -> Result<(), FileError> {
    let path_ref = Path::new(path);
    std::fs::create_dir(path_ref).map_err(|source| io_error("create_directory", path_ref, &source))
}

/// Removes the empty directory at `path` (PureBasic `DeleteDirectory`).
///
/// A non-empty directory is rejected by the OS; that failure is surfaced
/// unchanged as a typed [`FileError`] so a caller does not have to guess
/// whether the directory disappeared.
pub fn delete_directory(path: &str) -> Result<(), FileError> {
    let path_ref = Path::new(path);
    std::fs::remove_dir(path_ref).map_err(|source| io_error("delete_directory", path_ref, &source))
}

/// Lists the entries of `path`, sorted by name (PureBasic
/// `ExamineDirectory`).
///
/// The listing is sorted because `read_dir` yields entries in filesystem order,
/// which differs between machines and would make a script's output
/// non-deterministic. Symbolic links are reported as [`EntryKind::Other`]
/// rather than followed, so the call never stats outside the requested
/// directory.
pub fn examine_directory(path: &str) -> Result<Vec<DirEntry>, FileError> {
    let path_ref = Path::new(path);
    let read = std::fs::read_dir(path_ref)
        .map_err(|source| io_error("examine_directory", path_ref, &source))?;
    let mut entries = Vec::new();
    for entry in read {
        let entry = entry.map_err(|source| io_error("examine_directory", path_ref, &source))?;
        let file_type = entry
            .file_type()
            .map_err(|source| io_error("examine_directory", path_ref, &source))?;
        let kind = if file_type.is_dir() {
            EntryKind::Directory
        } else if file_type.is_file() {
            EntryKind::File
        } else {
            EntryKind::Other
        };
        entries.push(DirEntry {
            name: entry.file_name().to_string_lossy().into_owned(),
            kind,
        });
    }
    entries.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(entries)
}
