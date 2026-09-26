//! A unique temp directory that removes itself, so tests never touch user data.
//!
//! Shared by every integration test binary, so some helpers are unused in each
//! individual binary; that is expected and not a defect.

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

pub struct TempDir {
    path: PathBuf,
}

impl TempDir {
    pub fn create(label: &str) -> std::io::Result<Self> {
        let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("vr-pack-{label}-{}-{unique}", std::process::id()));
        std::fs::create_dir_all(&path)?;
        Ok(Self { path })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn join(&self, rel: impl AsRef<Path>) -> PathBuf {
        self.path.join(rel)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

/// A valid 1x1 32-bit `.ico`: a `BITMAPINFOHEADER`, one BGRA pixel and a padded
/// AND mask. It is only used as resource payload, so the pixels are arbitrary.
pub fn sample_ico() -> Vec<u8> {
    const ICONDIR_LEN: usize = 6;
    const ICONDIRENTRY_LEN: usize = 16;

    let mut dib = Vec::new();
    dib.extend_from_slice(&40u32.to_le_bytes()); // biSize
    dib.extend_from_slice(&1i32.to_le_bytes()); // biWidth
    dib.extend_from_slice(&2i32.to_le_bytes()); // biHeight: XOR + AND
    dib.extend_from_slice(&1u16.to_le_bytes()); // biPlanes
    dib.extend_from_slice(&32u16.to_le_bytes()); // biBitCount
    dib.extend_from_slice(&0u32.to_le_bytes()); // biCompression
    dib.extend_from_slice(&0u32.to_le_bytes()); // biSizeImage
    for _ in 0..4 {
        dib.extend_from_slice(&0i32.to_le_bytes());
    }
    dib.extend_from_slice(&[0, 0, 255, 255]); // one opaque red pixel (BGRA)
    dib.extend_from_slice(&[0, 0, 0, 0]); // AND mask, padded to 4 bytes

    let mut ico = Vec::new();
    ico.extend_from_slice(&0u16.to_le_bytes());
    ico.extend_from_slice(&1u16.to_le_bytes());
    ico.extend_from_slice(&1u16.to_le_bytes());
    ico.extend_from_slice(&[1, 1, 0, 0]); // width, height, colours, reserved
    ico.extend_from_slice(&1u16.to_le_bytes()); // planes
    ico.extend_from_slice(&32u16.to_le_bytes()); // bit count
    ico.extend_from_slice(&(dib.len() as u32).to_le_bytes());
    ico.extend_from_slice(&((ICONDIR_LEN + ICONDIRENTRY_LEN) as u32).to_le_bytes());
    ico.extend_from_slice(&dib);
    ico
}

/// Locates the runtime stub an integration test can patch or export onto: an
/// explicit `VR_RUNTIME_STUB`, else the debug/release `vr-runtime.exe` under the
/// active target directory. `None` means the test should print `SKIP`.
pub fn runtime_stub() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("VR_RUNTIME_STUB") {
        let path = PathBuf::from(path);
        return path.is_file().then_some(path);
    }

    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let target = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| manifest_dir.join("..").join("..").join("target"));

    for profile in ["debug", "release"] {
        let candidate = target.join(profile).join("vr-runtime.exe");
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}
