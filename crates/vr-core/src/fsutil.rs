//! File helpers shared by the project and settings stores.
//!
//! Writes go through a sibling temp file so a crash mid-write cannot truncate
//! the previous, valid state; [`CreatedPaths`] tracks whatever a failed
//! create/add left behind so callers can undo it.

use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Writes `bytes` to `path` via a temp file and an atomic rename.
pub(crate) fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let file_name = path.file_name().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "path has no file name component",
        )
    })?;
    let unique = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let temp = parent.join(format!(
        ".{}.{}.{}.tmp",
        file_name.to_string_lossy(),
        std::process::id(),
        unique
    ));

    std::fs::write(&temp, bytes)?;
    if let Err(err) = std::fs::rename(&temp, path) {
        let _ = std::fs::remove_file(&temp);
        return Err(err);
    }
    Ok(())
}

/// Paths created by a multi-step operation, remembered so a later failure can
/// remove exactly those and nothing pre-existing.
#[derive(Debug, Default)]
pub(crate) struct CreatedPaths {
    dirs: Vec<PathBuf>,
    files: Vec<PathBuf>,
}

impl CreatedPaths {
    /// Creates `path` (and parents) if missing, recording it for rollback.
    pub(crate) fn ensure_dir(&mut self, path: &Path) -> io::Result<()> {
        if path.is_dir() {
            return Ok(());
        }
        std::fs::create_dir_all(path)?;
        self.dirs.push(path.to_path_buf());
        Ok(())
    }

    /// Creates `path` with `bytes` if missing; leaves an existing file alone so
    /// repeated operations stay idempotent.
    pub(crate) fn ensure_file(&mut self, path: &Path, bytes: &[u8]) -> io::Result<()> {
        if path.exists() {
            return Ok(());
        }
        if let Some(parent) = path.parent() {
            self.ensure_dir(parent)?;
        }
        std::fs::write(path, bytes)?;
        self.files.push(path.to_path_buf());
        Ok(())
    }

    /// Deletes everything recorded so far, deepest first, ignoring failures
    /// because rollback runs on an error path already.
    pub(crate) fn rollback(&mut self) {
        for file in self.files.drain(..).rev() {
            let _ = std::fs::remove_file(file);
        }
        for dir in self.dirs.drain(..).rev() {
            let _ = std::fs::remove_dir(dir);
        }
    }
}
