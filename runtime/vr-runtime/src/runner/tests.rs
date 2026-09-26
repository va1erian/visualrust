//! Loader and runner tests over a synthetic `[prefix][bundle]` executable.

use super::*;
use crate::loader::{Extracted, load_from_file};
use crate::tempdir::TempDir;
use vr_core::manifest::{DbConfig, Manifest, NamedPath, Project, ProjectKind};
use vr_pack::Bundle;

fn console_manifest() -> Manifest {
    Manifest {
        project: Project {
            name: "demo".to_owned(),
            version: "0.1.0".to_owned(),
            kind: ProjectKind::Console,
            entry: "src/main.dyon".into(),
            description: None,
        },
        forms: Vec::new(),
        modules: vec![NamedPath {
            name: "util".to_owned(),
            path: "src/util.dyon".into(),
        }],
        assets: vec![NamedPath {
            name: "logo".to_owned(),
            path: "assets/logo.png".into(),
        }],
        extensions: Vec::new(),
        routes: Vec::new(),
        db: Some(DbConfig {
            path: "data/app.sqlite".into(),
        }),
    }
}

fn console_bundle() -> Bundle {
    let mut bundle = Bundle::new(console_manifest());
    bundle.add_source("entry", b"fn main() { }\n".to_vec());
    bundle.add_source("util", b"fn helper() { }\n".to_vec());
    bundle.add_asset("logo", vec![0x89, b'P', b'N', b'G']);
    bundle.set_database(vec![1, 2, 3, 4]);
    bundle
}

/// Writes `[arbitrary prefix bytes][bundle]` to a fresh file and returns the
/// enclosing temp dir (kept alive by the caller) and the executable path.
fn executable_with_bundle(label: &str) -> (TempDir, PathBuf) {
    let dir = TempDir::create(label).expect("temp dir");
    let path = dir.path().join("app.exe");
    std::fs::write(&path, b"MZ\x90\x00 arbitrary stub prefix").expect("write prefix");
    console_bundle()
        .append_to_file(&path)
        .expect("append bundle");
    (dir, path)
}

#[test]
fn run_embedded_from_locates_and_extracts_every_entry() {
    let (_dir, path) = executable_with_bundle("run");

    let report = run_embedded_from(&path).expect("the bundled console app runs");

    assert_eq!(report.kind, ProjectKind::Console);
    assert_eq!(report.entry, PathBuf::from("src/main.dyon"));
    assert_eq!(
        report.entries,
        vec![
            (EntryKind::Source, "entry".to_owned()),
            (EntryKind::Source, "util".to_owned()),
            (EntryKind::Asset, "logo".to_owned()),
            (EntryKind::Database, "database".to_owned()),
        ]
    );
}

#[test]
fn extraction_writes_real_paths_and_removes_them_on_drop() {
    let (_dir, path) = executable_with_bundle("extract");
    let root;

    {
        let extracted: Extracted = load_from_file(&path).expect("extract bundle");
        root = extracted.root().to_path_buf();

        let entry = std::fs::read_to_string(extracted.entry_path()).expect("entry written");
        assert_eq!(entry, "fn main() { }\n");
        assert!(extracted.path_for(EntryKind::Asset, "logo").is_some());
        assert!(extracted.path_for(EntryKind::Source, "util").is_some());
        assert!(extracted.path_for(EntryKind::Form, "main").is_none());
        assert!(root.join("data/app.sqlite").is_file());
    }

    assert!(!root.exists(), "the scratch tree is removed on drop");
}

#[test]
fn a_file_without_a_bundle_is_a_typed_no_bundle_error() {
    let dir = TempDir::create("nobundle").expect("temp dir");
    let path = dir.path().join("bare.exe");
    std::fs::write(&path, b"MZ\x90\x00 no footer here").expect("write bare stub");

    let error = run_embedded_from(&path).expect_err("a bare stub has no app");

    assert!(
        matches!(error, RuntimeError::NoBundle { .. }),
        "got {error:?}"
    );
}

#[test]
fn a_web_project_is_a_typed_not_yet_error() {
    let dir = TempDir::create("web").expect("temp dir");
    let path = dir.path().join("web.exe");
    let mut manifest = console_manifest();
    manifest.project.kind = ProjectKind::Web;
    manifest.routes = Vec::new();
    let mut bundle = Bundle::new(manifest);
    bundle.add_source("entry", b"fn main() { }\n".to_vec());
    std::fs::write(&path, b"MZ\x90\x00 stub").expect("write prefix");
    bundle.append_to_file(&path).expect("append bundle");

    let error = run_embedded_from(&path).expect_err("web is not hosted yet");

    assert!(
        matches!(error, RuntimeError::UnsupportedKind { kind: "web" }),
        "got {error:?}"
    );
}
