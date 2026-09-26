//! Select the host for a project kind and run the extracted entry point.

use std::path::{Path, PathBuf};

use vr_core::manifest::ProjectKind;
use vr_pack::EntryKind;

use crate::app::RuntimeApp;
use crate::error::RuntimeError;
use crate::loader::{Extracted, load_from_file};

/// What a completed run leaves behind: the kind hosted, the entry that ran, and
/// the entries that were extracted for it. Paths are not included because the
/// scratch tree is removed when the run returns.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunReport {
    pub kind: ProjectKind,
    pub entry: PathBuf,
    pub entries: Vec<(EntryKind, String)>,
}

/// Locates the bundle appended to `path`, extracts it, and runs the manifest
/// entry through the host selected by the project `type`.
pub fn run_embedded_from(path: impl AsRef<Path>) -> Result<RunReport, RuntimeError> {
    run_extracted(load_from_file(path)?)
}

/// Runs the bundle appended to the running executable. This is what the stub
/// calls at startup; resolving `current_exe` at run time keeps working after
/// the executable is renamed or moved.
pub fn run_embedded() -> Result<RunReport, RuntimeError> {
    let exe = std::env::current_exe().map_err(RuntimeError::CurrentExe)?;
    run_embedded_from(exe)
}

fn run_extracted(extracted: Extracted) -> Result<RunReport, RuntimeError> {
    let kind = extracted.manifest().project.kind;
    let entries = extracted
        .entries()
        .iter()
        .map(|e| (e.kind, e.name.clone()))
        .collect();

    match kind {
        // Desktop and console share a host today: both compile the entry and
        // run its `main`; a desktop program opens a window from inside it and a
        // console program simply returns. Web gets a typed "not yet" until the
        // HTTP server lands (#86/#87).
        ProjectKind::Desktop | ProjectKind::Console => run_script(&extracted)?,
        ProjectKind::Web => return Err(RuntimeError::UnsupportedKind { kind: "web" }),
    }

    Ok(RunReport {
        kind,
        entry: extracted.manifest().project.entry.clone(),
        entries,
    })
}

fn run_script(extracted: &Extracted) -> Result<(), RuntimeError> {
    let entry = extracted.entry_path();
    let source = std::fs::read_to_string(entry).map_err(|source| RuntimeError::ReadEntry {
        path: entry.to_path_buf(),
        source,
    })?;
    let mut app = RuntimeApp::from_source(&entry.to_string_lossy(), &source)?;
    app.run_main()?;
    Ok(())
}

#[cfg(test)]
mod tests;
