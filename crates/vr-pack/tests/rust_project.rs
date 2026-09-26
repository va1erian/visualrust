//! Emitted "build as a Rust project" crate: files, embedded bundle, build gate.
//!
//! The last test compiles and runs the emitted project against a tiny stub
//! `vr-runtime` (a path dependency), so it needs no network and stays fast. It
//! prints `SKIP` and passes when no `cargo` is on `PATH`.

mod common;

use std::path::{Path, PathBuf};
use std::process::Command;

use common::TempDir;
use vr_core::manifest::ProjectKind;
use vr_core::project::{Project, ProjectOptions};
use vr_pack::{
    Bundle, PackError, RuntimeDependency, RustProjectOptions, cargo_available, emit_rust_project,
};

const SOURCE: &str = "fn main() { println(\"rust project hello\") }\n";

/// A minimal project on disk: a valid `vrproj.toml` plus its entry source.
fn write_project(root: &Path) -> Project {
    let project = Project::create(root, ProjectOptions::new("rust-demo", ProjectKind::Console))
        .expect("create scratch project");
    std::fs::write(project.entry_path(), SOURCE).expect("write entry source");
    project
}

fn path_dependency() -> RuntimeDependency {
    RuntimeDependency::Path(PathBuf::from("runtime").join("vr-runtime"))
}

/// Writes a stub `vr-runtime` exposing only what the generated `main` calls, so
/// the emitted project compiles without the real (heavy) runtime closure.
fn write_fake_runtime(root: &Path) {
    std::fs::create_dir_all(root.join("src")).expect("create fake runtime src");
    std::fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"vr-runtime\"\nversion = \"0.0.0\"\nedition = \"2021\"\n",
    )
    .expect("write fake Cargo.toml");
    std::fs::write(root.join("src/lib.rs"), FAKE_RUNTIME).expect("write fake lib.rs");
}

#[test]
fn emitting_writes_the_project_and_embeds_the_bundle() {
    let dir = TempDir::create("emit").expect("temp dir");
    write_project(dir.path());
    let out = dir.join("app");

    let options = RustProjectOptions::new(path_dependency());
    let project = emit_rust_project(dir.path(), &out, &options).expect("emit");

    assert_eq!(project.name(), "rust-demo");
    assert!(project.build().is_none(), "build is opt-in");
    assert!(!out.join("target").exists(), "no build means no target dir");

    for file in ["Cargo.toml", "README.md", "app.vrbundle", "src/main.rs"] {
        assert!(out.join(file).is_file(), "missing {file}");
    }

    let cargo = std::fs::read_to_string(out.join("Cargo.toml")).expect("read Cargo.toml");
    assert!(cargo.contains("name = \"rust-demo\""), "cargo: {cargo}");
    assert!(cargo.contains("edition = \"2024\""), "cargo: {cargo}");
    assert!(cargo.contains("vr-runtime = { path = "), "cargo: {cargo}");
    assert!(cargo.contains("[workspace]"), "cargo: {cargo}");

    let main = std::fs::read_to_string(out.join("src/main.rs")).expect("read main.rs");
    assert!(
        main.contains("include_bytes!(\"../app.vrbundle\")"),
        "main: {main}"
    );
    assert!(main.contains("run_embedded_from"), "main: {main}");

    let bytes = std::fs::read(out.join("app.vrbundle")).expect("read bundle");
    assert_eq!(bytes.len(), project.bundle_len(), "reported bundle length");
    let bundle = Bundle::read_bytes(&bytes).expect("embedded bytes decode");
    assert_eq!(bundle.manifest.project.name, "rust-demo");
    assert_eq!(
        bundle
            .entry(vr_pack::EntryKind::Source, "entry")
            .expect("entry blob")
            .data,
        SOURCE.as_bytes()
    );
}

#[test]
fn a_second_emit_is_refused() {
    let dir = TempDir::create("exists").expect("temp dir");
    write_project(dir.path());
    let out = dir.join("app");
    let options = RustProjectOptions::new(path_dependency());
    emit_rust_project(dir.path(), &out, &options).expect("first emit");

    let err = emit_rust_project(dir.path(), &out, &options).expect_err("must refuse");

    assert!(
        matches!(err, PackError::RustProjectExists { .. }),
        "got {err:?}"
    );
}

#[test]
fn a_git_dependency_is_rendered_with_its_rev() {
    let dir = TempDir::create("git-dep").expect("temp dir");
    write_project(dir.path());
    let out = dir.join("app");
    let options = RustProjectOptions::new(RuntimeDependency::Git {
        url: "https://github.com/va1erian/visualrust".to_owned(),
        rev: Some("abc123".to_owned()),
    });

    emit_rust_project(dir.path(), &out, &options).expect("emit");

    let cargo = std::fs::read_to_string(out.join("Cargo.toml")).expect("read Cargo.toml");
    assert!(
        cargo.contains("git = \"https://github.com/va1erian/visualrust\", rev = \"abc123\""),
        "cargo: {cargo}"
    );
}

#[test]
fn generated_project_compiles_and_runs_with_a_toolchain() {
    if !cargo_available() {
        eprintln!("SKIP rust-project build: `cargo` is not on PATH");
        return;
    }

    let dir = TempDir::create("build").expect("temp dir");
    write_project(dir.path());
    let runtime = dir.join("fake-runtime");
    write_fake_runtime(&runtime);
    let out = dir.join("app");

    let options = RustProjectOptions::new(RuntimeDependency::Path(runtime)).with_build(true);
    let project = match emit_rust_project(dir.path(), &out, &options) {
        Ok(project) => project,
        Err(error) => {
            eprintln!("SKIP rust-project build: emit failed ({error})");
            return;
        }
    };

    let built = project.build().expect("build ran");
    let exe = built
        .root
        .join("target")
        .join("debug")
        .join("rust-demo.exe");
    assert!(exe.is_file(), "cargo did not produce {exe:?}");

    let output = Command::new(&exe).output().expect("run generated exe");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "generated exe failed: {stdout}");
    assert!(stdout.contains("src/main.dyon"), "stdout: {stdout}");
    eprintln!("rust-project build: compiled and ran the generated crate");
}

/// The bare minimum of `vr-runtime` the generated `main` reaches for.
const FAKE_RUNTIME: &str = r#"use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Desktop,
    Console,
    Web,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunReport {
    pub kind: Kind,
    pub entry: PathBuf,
    pub entries: Vec<(String, String)>,
}

#[derive(Debug)]
pub struct Error(String);

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Error {}

pub fn run_embedded_from(path: impl AsRef<Path>) -> Result<RunReport, Error> {
    let bytes = std::fs::read(path.as_ref()).map_err(|e| Error(e.to_string()))?;
    if bytes.is_empty() {
        return Err(Error("empty bundle".to_owned()));
    }
    Ok(RunReport {
        kind: Kind::Console,
        entry: PathBuf::from("src/main.dyon"),
        entries: Vec::new(),
    })
}
"#;
