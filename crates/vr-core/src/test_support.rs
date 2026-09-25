//! Test-only scaffolding: a unique directory under the OS temp dir.
//!
//! Tests must never touch real user data, so every store in these tests is
//! rooted here and the directory removes itself on drop.

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static COUNTER: AtomicU64 = AtomicU64::new(0);

pub(crate) struct TempDir {
    path: PathBuf,
}

impl TempDir {
    /// Allocates a not-yet-created directory with a collision-resistant name.
    pub(crate) fn new(label: &str) -> Self {
        let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let path = std::env::temp_dir().join(format!(
            "vr-core-{label}-{}-{nanos}-{unique}",
            std::process::id()
        ));
        Self { path }
    }

    /// Creates the directory and returns it, so an operation can target it.
    pub(crate) fn create(label: &str) -> std::io::Result<Self> {
        let dir = Self::new(label);
        std::fs::create_dir_all(&dir.path)?;
        Ok(dir)
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    pub(crate) fn join(&self, rel: impl AsRef<Path>) -> PathBuf {
        self.path.join(rel)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}
