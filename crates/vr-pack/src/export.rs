//! Export flow: turn a project directory plus a runtime stub into one `.exe`.
//!
//! The assembled app is a byte-for-byte copy of a prebuilt `vr-runtime.exe`
//! with a [`Bundle`] appended at EOF, which is exactly what the loader in #51
//! expects. Nothing here patches the PE image: icon and version resources are
//! #53, and the "build as a Rust project" mode is #70.
//!
//! Stub selection is driven by the manifest's project `type`. The location of
//! each stub is supplied through [`StubSource`] rather than hard-coded, so a
//! test can hand in a freshly built binary and the IDE can hand in whatever
//! discovery policy it adopts. A type with no stub is a typed error, never a
//! guess at a path.

use std::path::{Path, PathBuf};

use vr_core::manifest::{Manifest, ProjectKind};
use vr_core::project::MANIFEST_FILE;

use crate::bundle::Bundle;
use crate::error::PackError;

/// Where the runtime stub for a project type comes from.
///
/// Implementors own discovery (a build directory, bundled resources, a test
/// fixture); the export flow only asks for a path and validates it before use.
pub trait StubSource {
    /// The stub for `kind`, or `None` when this source has none for that type.
    fn stub_for(&self, kind: ProjectKind) -> Option<PathBuf>;
}

/// A [`StubSource`] holding one optional path per [`ProjectKind`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Stubs {
    pub desktop: Option<PathBuf>,
    pub console: Option<PathBuf>,
    pub web: Option<PathBuf>,
}

impl Stubs {
    /// Records the stub for `kind`, replacing any previous one.
    #[must_use]
    pub fn with(mut self, kind: ProjectKind, path: impl Into<PathBuf>) -> Self {
        match kind {
            ProjectKind::Desktop => self.desktop = Some(path.into()),
            ProjectKind::Console => self.console = Some(path.into()),
            ProjectKind::Web => self.web = Some(path.into()),
        }
        self
    }
}

impl StubSource for Stubs {
    fn stub_for(&self, kind: ProjectKind) -> Option<PathBuf> {
        match kind {
            ProjectKind::Desktop => self.desktop.clone(),
            ProjectKind::Console => self.console.clone(),
            ProjectKind::Web => self.web.clone(),
        }
    }
}

/// Exports `project_dir` using the stub its manifest type selects.
///
/// The manifest is loaded (and therefore validated) from
/// `project_dir/vrproj.toml`; every file it references is read into the bundle.
/// Returns the path written on success.
pub fn export(
    project_dir: impl AsRef<Path>,
    stubs: &impl StubSource,
    output: impl AsRef<Path>,
) -> Result<PathBuf, PackError> {
    let project_dir = project_dir.as_ref();
    let manifest = Manifest::load(project_dir.join(MANIFEST_FILE))?;
    let stub = stubs
        .stub_for(manifest.project.kind)
        .ok_or(PackError::StubUnavailable {
            kind: kind_name(manifest.project.kind),
        })?;
    assemble(project_dir, manifest, &stub, output.as_ref())
}

/// Exports with an explicit stub, bypassing type-based selection.
///
/// Callers that already resolved a stub (or tests that exercise the append step
/// without a real runtime) use this directly. The project manifest still drives
/// what is bundled.
pub fn export_with_stub(
    project_dir: impl AsRef<Path>,
    stub: impl AsRef<Path>,
    output: impl AsRef<Path>,
) -> Result<PathBuf, PackError> {
    let project_dir = project_dir.as_ref();
    let manifest = Manifest::load(project_dir.join(MANIFEST_FILE))?;
    assemble(project_dir, manifest, stub.as_ref(), output.as_ref())
}

fn assemble(
    project_dir: &Path,
    manifest: Manifest,
    stub: &Path,
    output: &Path,
) -> Result<PathBuf, PackError> {
    // Refuse to clobber before doing any read work, so a failed export never
    // destroys an existing build artifact.
    if output.exists() {
        return Err(PackError::OutputExists {
            path: output.to_path_buf(),
        });
    }
    if !stub.is_file() {
        return Err(PackError::StubMissing {
            path: stub.to_path_buf(),
        });
    }

    let bundle = Bundle::from_project(project_dir, manifest)?;
    std::fs::copy(stub, output).map_err(|source| PackError::StubCopy {
        from: stub.to_path_buf(),
        to: output.to_path_buf(),
        source,
    })?;
    bundle.append_to_file(output)?;
    Ok(output.to_path_buf())
}

fn kind_name(kind: ProjectKind) -> &'static str {
    match kind {
        ProjectKind::Desktop => "desktop",
        ProjectKind::Console => "console",
        ProjectKind::Web => "web",
    }
}
