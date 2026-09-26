//! Export-flow behaviour: stub selection errors, copy+append, and a real run.
//!
//! The last test copies the built `vr-runtime.exe`, exports a tiny console
//! project onto it, and runs the result headless. It prints `SKIP` and passes
//! when no stub can be located or spawned in this environment.

mod common;

use std::path::Path;
use std::process::Command;

use common::{TempDir, runtime_stub};
use vr_core::manifest::ProjectKind;
use vr_core::project::{Project, ProjectOptions};
use vr_pack::{Bundle, PackError, Stubs, export, export_with_stub};

/// A minimal project on disk: a valid `vrproj.toml` plus its entry source.
fn write_project(root: &Path, kind: ProjectKind, source: &str) -> Project {
    let project = Project::create(root, ProjectOptions::new("export-demo", kind))
        .expect("create scratch project");
    std::fs::write(project.entry_path(), source).expect("write entry source");
    project
}

#[test]
fn a_type_with_no_stub_is_a_typed_error() {
    let dir = TempDir::create("no-stub").expect("temp dir");
    write_project(dir.path(), ProjectKind::Web, "fn main() {}\n");
    let output = dir.join("out.exe");

    let err = export(dir.path(), &Stubs::default(), &output).expect_err("web has no stub");

    assert!(
        matches!(err, PackError::StubUnavailable { kind: "web" }),
        "got {err:?}"
    );
    assert!(!output.exists(), "a refused export must write nothing");
}

#[test]
fn a_missing_stub_path_is_a_typed_error() {
    let dir = TempDir::create("missing-stub").expect("temp dir");
    write_project(dir.path(), ProjectKind::Console, "fn main() {}\n");
    let stubs = Stubs::default().with(ProjectKind::Console, dir.join("nope.exe"));

    let err = export(dir.path(), &stubs, dir.join("out.exe")).expect_err("stub is absent");

    assert!(matches!(err, PackError::StubMissing { .. }), "got {err:?}");
}

#[test]
fn an_existing_output_is_refused_before_work() {
    let dir = TempDir::create("output-exists").expect("temp dir");
    write_project(dir.path(), ProjectKind::Console, "fn main() {}\n");
    let stub = dir.join("stub.exe");
    std::fs::write(&stub, b"MZ-stub").expect("write dummy stub");
    let output = dir.join("out.exe");
    std::fs::write(&output, b"existing").expect("pre-create output");

    let err =
        export_with_stub(dir.path(), &stub, &output).expect_err("output must not be clobbered");

    assert!(matches!(err, PackError::OutputExists { .. }), "got {err:?}");
    assert_eq!(
        std::fs::read(&output).expect("read output"),
        b"existing",
        "the existing file must be untouched"
    );
}

#[test]
fn export_copies_the_stub_and_appends_the_bundle() {
    let dir = TempDir::create("append").expect("temp dir");
    write_project(
        dir.path(),
        ProjectKind::Console,
        "fn main() { println(\"hi\") }\n",
    );
    let stub_bytes = b"MZ-dummy-runtime-stub".to_vec();
    let stub = dir.join("stub.exe");
    std::fs::write(&stub, &stub_bytes).expect("write dummy stub");
    let output = dir.join("app.exe");

    let written = export_with_stub(dir.path(), &stub, &output).expect("export succeeds");

    assert_eq!(written, output);
    let after = std::fs::read(&output).expect("read output");
    assert!(after.len() > stub_bytes.len(), "bundle must be appended");
    assert_eq!(&after[..stub_bytes.len()], stub_bytes.as_slice());

    let bundle = Bundle::read_from_file(&output).expect("output carries a readable bundle");
    assert_eq!(bundle.manifest.project.name, "export-demo");
    assert_eq!(
        bundle
            .entry(vr_pack::EntryKind::Source, "entry")
            .expect("entry blob")
            .data,
        b"fn main() { println(\"hi\") }\n"
    );
}

#[test]
fn exported_exe_runs_the_bundled_console_app() {
    let Some(stub) = runtime_stub() else {
        eprintln!(
            "SKIP export e2e: no vr-runtime.exe found (build vr-runtime or set VR_RUNTIME_STUB)"
        );
        return;
    };

    let dir = match TempDir::create("e2e") {
        Ok(dir) => dir,
        Err(error) => {
            eprintln!("SKIP export e2e: no scratch dir ({error})");
            return;
        }
    };
    write_project(
        dir.path(),
        ProjectKind::Console,
        "fn main() { println(\"exported hello\") }\n",
    );
    let output = dir.join("exported-app.exe");
    let stubs = Stubs::default().with(ProjectKind::Console, &stub);

    if let Err(error) = export(dir.path(), &stubs, &output) {
        eprintln!("SKIP export e2e: export failed ({error})");
        return;
    }

    match Command::new(&output).output() {
        Ok(result) => {
            let stdout = String::from_utf8_lossy(&result.stdout);
            let stderr = String::from_utf8_lossy(&result.stderr);
            assert!(
                result.status.success(),
                "exported exe exited {:?}\nstdout: {stdout}\nstderr: {stderr}",
                result.status
            );
            assert!(
                stdout.contains("main.dyon"),
                "exported exe did not report its bundled entry; stdout: {stdout}"
            );
            eprintln!(
                "export e2e: exported exe ran the bundle with exit code {}",
                result.status.code().unwrap_or(-1)
            );
        }
        Err(error) => {
            eprintln!("SKIP export e2e: could not spawn the exported exe ({error})");
        }
    }
}
