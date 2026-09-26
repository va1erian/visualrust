//! A unique per-run scratch directory that deletes itself.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::error::RuntimeError;

static COUNTER: AtomicU64 = AtomicU64::new(0);

/// Owns the extraction root for one run. Cleanup runs on `Drop`, so it fires
/// on an early extraction error and not only on a clean exit.
pub(crate) struct TempDir {
    path: PathBuf,
}

impl TempDir {
    pub(crate) fn create(label: &str) -> Result<Self, RuntimeError> {
        // A counter keeps concurrent runs in the same process from colliding;
        // the pid keeps separate processes apart.
        let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "vr-runtime-{label}-{}-{unique}",
            std::process::id()
        ));
        std::fs::create_dir_all(&path).map_err(|source| RuntimeError::TempDir {
            path: path.clone(),
            action: "created",
            source,
        })?;
        Ok(Self { path })
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        // Best effort: a drop cannot report failure, and a leftover scratch dir
        // is not something the app can act on. Windows keeps no lock on the
        // extracted files once their handles are closed.
        let _ = std::fs::remove_dir_all(&self.path);
    }
}
