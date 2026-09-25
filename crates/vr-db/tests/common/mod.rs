//! A temp database file that deletes itself, so tests never touch user data.

use std::path::{Path, PathBuf};

pub struct TempDb {
    path: PathBuf,
}

impl TempDb {
    pub fn new(tag: &str) -> Self {
        let path = std::env::temp_dir().join(format!("vr-db-{tag}-{}.sqlite", std::process::id()));
        let _ = std::fs::remove_file(&path);
        Self { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempDb {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}
