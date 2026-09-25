//! Round-trip, corruption and PE-append behaviour for the bundle format.

mod common;

use common::TempDir;
use vr_core::manifest::{DbConfig, Manifest, NamedPath, Project, ProjectKind};
use vr_pack::{Bundle, EntryKind, PackError};

fn sample_manifest() -> Manifest {
    Manifest {
        project: Project {
            name: "demo".to_owned(),
            version: "0.1.0".to_owned(),
            kind: ProjectKind::Desktop,
            entry: "src/main.dyon".into(),
            description: Some("bundle format test".to_owned()),
        },
        forms: vec![NamedPath {
            name: "main".to_owned(),
            path: "forms/main.vrform".into(),
        }],
        modules: vec![NamedPath {
            name: "util".to_owned(),
            path: "src/util.dyon".into(),
        }],
        assets: vec![NamedPath {
            name: "logo".to_owned(),
            path: "assets/logo.png".into(),
        }],
        extensions: vec![NamedPath {
            name: "native".to_owned(),
            path: "ext/native.dll".into(),
        }],
        routes: Vec::new(),
        db: Some(DbConfig {
            path: "data/app.sqlite".into(),
        }),
    }
}

fn sample_bundle() -> Bundle {
    let mut bundle = Bundle::new(sample_manifest());
    bundle.add_source("entry", br#"fn main() { println("hi") }"#.to_vec());
    bundle.add_source("util", b"fn helper() {}".to_vec());
    bundle.add_form("main", b"form-main".to_vec());
    bundle.add_asset("logo", vec![0x89, b'P', b'N', b'G']);
    bundle.add_extension("native", b"MZ-native-dll".to_vec());
    bundle.set_database(vec![1, 2, 3, 4, 5]);
    bundle
}

#[test]
fn bytes_round_trip_preserves_manifest_and_every_entry() {
    let bundle = sample_bundle();
    let bytes = bundle.to_bytes().expect("encode bundle");

    let read = Bundle::read_bytes(&bytes).expect("decode bundle");
    assert_eq!(read, bundle);
    assert_eq!(read.manifest, sample_manifest());
    assert_eq!(read.entries().len(), 6);
    assert_eq!(read.database(), Some([1, 2, 3, 4, 5].as_slice()));
    assert!(read.entry(EntryKind::Extension, "native").is_some());
    assert!(read.entry(EntryKind::Form, "main").is_some());
}

#[test]
fn file_round_trip_preserves_content() {
    let dir = TempDir::create("roundtrip").expect("temp dir");
    let path = dir.join("app.vrb");

    let bundle = sample_bundle();
    bundle.write_to_file(&path).expect("write bundle");
    let read = Bundle::read_from_file(&path).expect("read bundle");

    assert_eq!(read, bundle);
}

#[test]
fn flipped_payload_byte_fails_the_hash() {
    let mut bytes = sample_bundle().to_bytes().expect("encode bundle");
    bytes[0] ^= 0xff;

    let err = Bundle::read_bytes(&bytes).expect_err("flip must be caught");
    assert!(matches!(err, PackError::HashMismatch), "got {err:?}");
}

#[test]
fn truncated_bundle_is_a_typed_error() {
    let bytes = sample_bundle().to_bytes().expect("encode bundle");
    let truncated = &bytes[..bytes.len() - 1];

    let err = Bundle::read_bytes(truncated).expect_err("truncation must be caught");
    assert!(matches!(err, PackError::MissingFooter), "got {err:?}");
}

#[test]
fn corrupted_footer_magic_is_reported() {
    let mut bytes = sample_bundle().to_bytes().expect("encode bundle");
    let footer_start = bytes.len() - vr_pack::format::FOOTER_LEN;
    bytes[footer_start] ^= 0xff;

    let err = Bundle::read_bytes(&bytes).expect_err("bad magic must be caught");
    assert!(matches!(err, PackError::MissingFooter), "got {err:?}");
}

#[test]
fn corrupt_digest_is_reported_without_decompressing() {
    let mut bytes = sample_bundle().to_bytes().expect("encode bundle");
    let digest_start = bytes.len() - 32;
    bytes[digest_start] ^= 0xff;

    let err = Bundle::read_bytes(&bytes).expect_err("bad digest must be caught");
    assert!(matches!(err, PackError::HashMismatch), "got {err:?}");
}

#[test]
fn from_project_reads_every_manifest_reference() {
    let dir = TempDir::create("project").expect("temp dir");
    for (rel, body) in [
        ("src/main.dyon", "fn main() {}"),
        ("src/util.dyon", "fn helper() {}"),
        ("forms/main.vrform", "form"),
        ("assets/logo.png", "png"),
        ("ext/native.dll", "dll"),
        ("data/app.sqlite", "sqlite"),
    ] {
        let path = dir.join(rel);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("create dir");
        std::fs::write(path, body).expect("write fixture");
    }

    let bundle = Bundle::from_project(dir.path(), sample_manifest()).expect("pack project");

    assert_eq!(bundle.manifest.project.name, "demo");
    assert_eq!(bundle.entries().len(), 6);
    assert_eq!(
        bundle
            .entry(EntryKind::Source, "entry")
            .expect("entry")
            .data,
        b"fn main() {}"
    );
    assert_eq!(bundle.database(), Some(b"sqlite".as_slice()));
}

#[test]
fn missing_project_file_is_a_typed_error() {
    let dir = TempDir::create("missing").expect("temp dir");
    let err = Bundle::from_project(dir.path(), sample_manifest()).expect_err("must fail");
    assert!(matches!(err, PackError::ReadFile { .. }), "got {err:?}");
}

#[test]
fn appending_to_an_executable_does_not_disturb_its_bytes() {
    let dir = TempDir::create("pe").expect("temp dir");
    let path = dir.join("app.exe");
    let original = execution_stub();
    std::fs::write(&path, &original).expect("write stub");
    assert_eq!(&original[0..2], b"MZ", "fixture must look like a PE image");

    let bundle = sample_bundle();
    bundle.append_to_file(&path).expect("append bundle");

    let after = std::fs::read(&path).expect("read stub back");
    assert!(after.len() > original.len());
    assert_eq!(
        &after[..original.len()],
        original.as_slice(),
        "prefix bytes must be untouched"
    );

    let read = Bundle::read_from_file(&path).expect("reader must find the appended bundle");
    assert_eq!(read, bundle);
}

/// Prefer the real runtime stub (which proves the footer goes after a true PE
/// image) and fall back to a synthetic `MZ`/`PE` header when it is not built.
fn execution_stub() -> Vec<u8> {
    if let Some(bytes) = find_runtime_stub() {
        return bytes;
    }
    let mut bytes = vec![0u8; 4096];
    bytes[0] = b'M';
    bytes[1] = b'Z';
    bytes[0x3c..0x40].copy_from_slice(&0x80u32.to_le_bytes());
    bytes[0x80..0x84].copy_from_slice(b"PE\0\0");
    bytes
}

fn find_runtime_stub() -> Option<Vec<u8>> {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let target = std::env::var_os("CARGO_TARGET_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| manifest_dir.join("..").join("..").join("target"));

    for profile in ["debug", "release"] {
        let candidate = target.join(profile).join("vr-runtime.exe");
        if let Ok(bytes) = std::fs::read(&candidate) {
            return Some(bytes);
        }
    }
    None
}
