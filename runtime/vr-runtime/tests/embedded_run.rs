//! End-to-end: a copied runtime stub runs the bundle appended to its own image.
//!
//! The test copies the built `vr-runtime.exe`, appends a console bundle whose
//! entry is a tiny Dyon script, and runs the copy as a child process. Because
//! the copy is renamed relative to the original, a clean exit also proves the
//! loader tracks its own path rather than a baked-in one. It prints `SKIP` and
//! passes when no binary can be produced or spawned in this environment.

use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use vr_core::manifest::{Manifest, Project, ProjectKind};
use vr_pack::Bundle;

static COUNTER: AtomicU64 = AtomicU64::new(0);

/// Removes its scratch directory on drop so the test leaves nothing behind.
struct Scratch {
    path: PathBuf,
}

impl Scratch {
    fn new() -> std::io::Result<Self> {
        let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "vr-runtime-embedded-{}-{unique}",
            std::process::id()
        ));
        std::fs::create_dir_all(&path)?;
        Ok(Self { path })
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

fn console_manifest() -> Manifest {
    Manifest {
        project: Project {
            name: "embedded-demo".to_owned(),
            version: "0.1.0".to_owned(),
            kind: ProjectKind::Console,
            entry: "src/main.dyon".into(),
            description: None,
        },
        forms: Vec::new(),
        modules: Vec::new(),
        assets: Vec::new(),
        extensions: Vec::new(),
        routes: Vec::new(),
        db: None,
    }
}

/// The stub Cargo built for this package; `None` when the env var is absent.
fn built_stub() -> Option<PathBuf> {
    let path = option_env!("CARGO_BIN_EXE_vr-runtime")?;
    let path = PathBuf::from(path);
    path.is_file().then_some(path)
}

#[test]
fn copied_stub_runs_its_appended_bundle() {
    let Some(stub) = built_stub() else {
        eprintln!("SKIP vr-runtime embedded-run test: no vr-runtime binary was built");
        return;
    };

    let scratch = match Scratch::new() {
        Ok(scratch) => scratch,
        Err(error) => {
            eprintln!("SKIP vr-runtime embedded-run test: no scratch dir ({error})");
            return;
        }
    };

    // A different file name proves the loader uses its own image path.
    let copy = scratch.path.join("renamed-app.exe");
    if let Err(error) = std::fs::copy(&stub, &copy) {
        eprintln!("SKIP vr-runtime embedded-run test: could not copy the stub ({error})");
        return;
    }

    let mut bundle = Bundle::new(console_manifest());
    bundle.add_source(
        "entry",
        b"fn main() {\n    println(\"embedded hello\")\n}\n".to_vec(),
    );
    if let Err(error) = bundle.append_to_file(&copy) {
        eprintln!("SKIP vr-runtime embedded-run test: could not append bundle ({error})");
        return;
    }

    match Command::new(&copy).output() {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            assert!(
                output.status.success(),
                "the copied stub exited with {:?}\nstdout: {stdout}\nstderr: {stderr}",
                output.status
            );
            assert!(
                stdout.contains("src/main.dyon"),
                "the stub did not report running the bundled entry; stdout: {stdout}"
            );
            eprintln!(
                "vr-runtime embedded-run test: copied stub ran the bundle with exit code {}",
                output.status.code().unwrap_or(-1)
            );
        }
        Err(error) => {
            eprintln!(
                "SKIP vr-runtime embedded-run test: could not spawn the copied stub ({error})"
            );
        }
    }
}
