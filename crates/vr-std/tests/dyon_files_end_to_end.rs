//! A Dyon script exercises the file and directory commands through the shared
//! runtime, proving `vr_std::register` wires them into `from_source_with`.
//!
//! The script runs inside a fresh temp directory handed to it as an argument, so
//! the suite never touches real user data.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use vr_dyon::{DyonRuntime, Variable};

/// An owned temp directory removed when the test ends.
struct TempDir {
    path: PathBuf,
}

impl TempDir {
    fn new(tag: &str) -> Self {
        let path =
            std::env::temp_dir().join(format!("vr-std-files-e2e-{}-{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("create temp dir");
        Self { path }
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

/// Forward slashes work on Windows and need no escaping in a Dyon string.
const SCRIPT: &str = r#"
fn run(dir: str) -> str {
    text_path := dir + "/note.txt"
    writer := open_file(text_path, "write")
    write_string(writer, "Visual")
    close_file(writer)

    appender := open_file(text_path, "append")
    write_string(appender, "Rust")
    close_file(appender)

    reader := open_file(text_path, "read")
    contents := read_string(reader)
    size := file_size(reader)
    at_end := eof(reader)
    close_file(reader)

    bin_path := dir + "/blob.bin"
    bin_writer := open_file(bin_path, "write")
    write_data(bin_writer, [86, 82])
    close_file(bin_writer)
    bin_reader := open_file(bin_path, "read")
    data := read_data(bin_reader)
    close_file(bin_reader)

    copy_path := dir + "/copy.txt"
    copied := copy_file(text_path, copy_path)
    delete_file(copy_path)

    sub := dir + "/sub"
    create_directory(sub)
    entries := examine_directory(dir)
    delete_directory(sub)

    return contents +
        "|" + str(size) +
        "|" + (if at_end { "eof" } else { "open" }) +
        "|" + str(data[0]) + str(data[1]) +
        "|" + str(copied) +
        "|" + entries[1].name +
        "|" + get_path_part(text_path, "name") +
        "|" + get_path_part(text_path, "ext")
}
fn main() {}
"#;

#[test]
fn dyon_script_uses_the_file_library() {
    let temp = TempDir::new("files");
    let mut runtime =
        DyonRuntime::from_source_with("files_end_to_end.dyon", SCRIPT, vr_std::register)
            .expect("program compiles with the native signatures");
    let argument = Variable::Str(Arc::new(temp.path.display().to_string()));
    let result: String = runtime.call_ret("run", &[argument]).expect("script runs");

    // At examine time the directory holds `blob.bin`, `note.txt` and `sub`
    // (sorted), so index 1 is `note.txt`; `copy.txt` was already deleted.
    // `len` is deliberately not used: the string library's `len` registration
    // replaces Dyon's array `len` once this module is registered.
    assert_eq!(result, "VisualRust|10|eof|8682|10|note.txt|note.txt|txt");
    assert!(!Path::new(&temp.path.join("copy.txt")).exists());
    assert!(!Path::new(&temp.path.join("sub")).exists());
}

#[test]
fn a_write_data_byte_out_of_range_is_a_runtime_error() {
    let temp = TempDir::new("bad-byte");
    let source = r#"
fn run(dir: str) -> str {
    writer := open_file(dir + "/bad.bin", "write")
    write_data(writer, [300])
    close_file(writer)
    return "unreachable"
}
fn main() {}
"#;
    let mut runtime =
        DyonRuntime::from_source_with("files_bad_byte.dyon", source, vr_std::register)
            .expect("program compiles");
    let argument = Variable::Str(Arc::new(temp.path.display().to_string()));
    assert!(runtime.call_ret::<String>("run", &[argument]).is_err());
}

#[test]
fn opening_a_missing_file_surfaces_as_a_runtime_error() {
    let temp = TempDir::new("missing");
    let source = r#"
fn run(dir: str) -> str {
    reader := open_file(dir + "/does-not-exist.txt", "read")
    return "unreachable"
}
fn main() {}
"#;
    let mut runtime = DyonRuntime::from_source_with("files_missing.dyon", source, vr_std::register)
        .expect("program compiles");
    let argument = Variable::Str(Arc::new(temp.path.display().to_string()));
    assert!(runtime.call_ret::<String>("run", &[argument]).is_err());
}
