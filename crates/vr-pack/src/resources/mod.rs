//! Patches a PE image's icon, version info and manifest resources.
//!
//! The heavy lifting is Win32 `BeginUpdateResourceW`/`UpdateResourceW`/
//! `EndUpdateResourceW`, wrapped by the unsafe [`crate::sys`] module. This
//! module stays safe: it validates and encodes the inputs, patches a *staging
//! copy* so a failure never corrupts the input, and only swaps the copy over the
//! target once the Win32 calls succeed.
//!
//! [`patch_resources`] is deliberately best-effort and re-exportable: the export
//! flow (#52) does not have to call it, and a caller that cannot patch an image
//! can still ship the unpatched executable.

mod icon;
mod version;

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use vr_core::manifest::Manifest;

use crate::error::PackError;
use crate::sys;

/// PE resource type ids from `WinUser.h`. Kept numeric so the safe layer never
/// touches `windows` types.
pub(crate) const RT_ICON: u16 = 3;
pub(crate) const RT_GROUP_ICON: u16 = 14;
pub(crate) const RT_VERSION: u16 = 16;
pub(crate) const RT_MANIFEST: u16 = 24;

/// The manifest and version resources are conventionally registered under this
/// language.
const LANG_EN_US: u16 = version::LANG_EN_US;

/// `CREATEPROCESS_MANIFEST_RESOURCE_ID` from `WinUser.h`.
const MANIFEST_ID: u16 = 1;
/// `VS_VERSION_INFO` is conventionally resource id 1.
const VERSION_ID: u16 = 1;

/// One resource write: type, numeric name id, language and the exact bytes to
/// store. Standard resources all use `MAKEINTRESOURCE` ids, so the name is a
/// `u16`; [`crate::sys`] turns it into the `PCWSTR` Win32 wants.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PatchOp {
    pub type_id: u16,
    pub name_id: u16,
    pub language: u16,
    pub data: Vec<u8>,
}

/// File and product identity written into `VS_VERSIONINFO`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionInfo {
    pub product_name: String,
    pub file_description: String,
    pub version: String,
}

impl VersionInfo {
    pub fn new(
        product_name: impl Into<String>,
        file_description: impl Into<String>,
        version: impl Into<String>,
    ) -> Self {
        Self {
            product_name: product_name.into(),
            file_description: file_description.into(),
            version: version.into(),
        }
    }

    /// Derives the fields from a project manifest: the product name and version
    /// come from the project, the description falls back to the name.
    pub fn from_manifest(manifest: &Manifest) -> Self {
        let product_name = manifest.project.name.clone();
        let file_description = manifest
            .project
            .description
            .clone()
            .unwrap_or_else(|| product_name.clone());
        Self {
            product_name,
            file_description,
            version: manifest.project.version.clone(),
        }
    }
}

/// The resource overrides to apply to an exported executable. Every field is
/// optional; an empty set makes [`patch_resources`] a no-op.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Resources {
    icon: Option<Vec<u8>>,
    version: Option<VersionInfo>,
    manifest: Option<Vec<u8>>,
}

impl Resources {
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the application icon from `.ico` file bytes.
    #[must_use]
    pub fn with_icon(mut self, ico: impl Into<Vec<u8>>) -> Self {
        self.icon = Some(ico.into());
        self
    }

    #[must_use]
    pub fn with_version(mut self, info: VersionInfo) -> Self {
        self.version = Some(info);
        self
    }

    /// Embeds an XML application manifest (`RT_MANIFEST`).
    #[must_use]
    pub fn with_manifest(mut self, xml: impl Into<Vec<u8>>) -> Self {
        self.manifest = Some(xml.into());
        self
    }

    pub fn is_empty(&self) -> bool {
        self.icon.is_none() && self.version.is_none() && self.manifest.is_none()
    }

    /// Encodes everything into the flat list of resource writes. Encoding
    /// happens before any file is touched, so a bad icon/version cannot leave a
    /// half-patched executable behind.
    fn encode(&self) -> Result<Vec<PatchOp>, PackError> {
        let mut ops = Vec::new();
        if let Some(ico) = &self.icon {
            ops.extend(icon::ops(ico)?);
        }
        if let Some(info) = &self.version {
            let fixed = version::parse_version(&info.version)?;
            let normalized = format!("{}.{}.{}.{}", fixed[0], fixed[1], fixed[2], fixed[3]);
            let fields = version::VersionFields {
                file_description: &info.file_description,
                product_name: &info.product_name,
                file_version: normalized.clone(),
                product_version: normalized,
                fixed,
            };
            ops.push(PatchOp {
                type_id: RT_VERSION,
                name_id: VERSION_ID,
                language: LANG_EN_US,
                data: version::build(&fields),
            });
        }
        if let Some(xml) = &self.manifest {
            ops.push(PatchOp {
                type_id: RT_MANIFEST,
                name_id: MANIFEST_ID,
                language: LANG_EN_US,
                data: xml.clone(),
            });
        }
        Ok(ops)
    }
}

/// Patches `exe` in place with `resources`, best-effort and typed.
///
/// The input is never corrupted: the bytes are copied to a staging file in the
/// same directory, that copy is patched, and only on success is it renamed over
/// `exe`. Any failure removes the staging file and leaves `exe` as it was. An
/// empty [`Resources`] returns `Ok(())` without touching the file.
pub fn patch_resources(exe: impl AsRef<Path>, resources: &Resources) -> Result<(), PackError> {
    let exe = exe.as_ref();
    if resources.is_empty() {
        return Ok(());
    }
    let ops = resources.encode()?;
    if !exe.is_file() {
        return Err(PackError::StubMissing {
            path: exe.to_path_buf(),
        });
    }

    let staging = staging_path(exe);
    std::fs::copy(exe, &staging).map_err(|source| PackError::StubCopy {
        from: exe.to_path_buf(),
        to: staging.clone(),
        source,
    })?;

    if let Err(message) = sys::patch_file(&staging, &ops) {
        let _ = std::fs::remove_file(&staging);
        return Err(PackError::ResourcePatch {
            path: exe.to_path_buf(),
            message,
        });
    }

    std::fs::rename(&staging, exe).map_err(|source| {
        let _ = std::fs::remove_file(&staging);
        PackError::ResourceCommit {
            path: exe.to_path_buf(),
            source,
        }
    })
}

/// A unique sibling path: same directory guarantees the final rename is a
/// same-volume move (atomic replace) rather than a cross-device copy.
fn staging_path(exe: &Path) -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
    let name = exe
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "image".to_owned());
    exe.with_file_name(format!(
        ".{name}.vrpatch-{}-{unique}.tmp",
        std::process::id()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use vr_core::manifest::{Manifest, Project, ProjectKind};

    fn manifest() -> Manifest {
        Manifest {
            project: Project {
                name: "demo-app".to_owned(),
                version: "1.4.2".to_owned(),
                kind: ProjectKind::Desktop,
                entry: "src/main.dyon".into(),
                description: Some("A demo".to_owned()),
            },
            forms: Vec::new(),
            modules: Vec::new(),
            assets: Vec::new(),
            extensions: Vec::new(),
            routes: Vec::new(),
            db: None,
        }
    }

    #[test]
    fn encode_builds_one_op_per_requested_resource() {
        let resources = Resources::new()
            .with_icon(icon::sample_ico())
            .with_version(VersionInfo::from_manifest(&manifest()))
            .with_manifest(b"<assembly/>");
        let ops = resources.encode().expect("encode");
        assert_eq!(ops.len(), 4, "two icon ops + version + manifest");
        assert!(ops.iter().any(|op| op.type_id == RT_VERSION));
        assert!(ops.iter().any(|op| op.type_id == RT_MANIFEST));
    }

    #[test]
    fn empty_resources_encode_to_nothing() {
        let ops = Resources::new().encode().expect("encode");
        assert!(ops.is_empty());
    }

    #[test]
    fn an_invalid_version_fails_before_touching_a_file() {
        let resources = Resources::new().with_version(VersionInfo::new("p", "p", "beta"));
        let err = resources.encode().expect_err("bad version");
        assert!(
            matches!(err, PackError::InvalidVersion { .. }),
            "got {err:?}"
        );
    }
}
