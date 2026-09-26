//! Unit tests for the pure file API.
//!
//! Every test works inside a [`TempDir`] under the OS temp directory and removes
//! it on drop, so the suite never reads or writes real user data. The directory
//! name is tagged with the process id and the test name to keep parallel tests
//! from sharing a path.

use std::path::{Path, PathBuf};

use super::*;
use crate::error::FileError;

/// An owned temporary directory removed when the test ends.
struct TempDir {
    path: PathBuf,
}

impl TempDir {
    fn new(tag: &str) -> Self {
        let path = std::env::temp_dir().join(format!("vr-std-files-{}-{tag}", std::process::id()));
        // Clear a leftover from a previously aborted run before creating it.
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("create temp dir");
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn file(&self, name: &str) -> PathBuf {
        self.path.join(name)
    }

    fn text(&self, path: &Path) -> String {
        path.display().to_string()
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

/// Opens for write, writes `text`, and closes, returning the path.
fn write_fixture(temp: &TempDir, name: &str, text: &str) -> String {
    let path = temp.text(&temp.file(name));
    let mut handle = open_file(&path, "write").expect("open for write");
    write_string(&mut handle, text).expect("write");
    close_file(&mut handle);
    path
}

#[test]
fn write_then_read_string_round_trips() {
    let temp = TempDir::new("round-trip");
    let path = write_fixture(&temp, "hello.txt", "hello world");

    let mut handle = open_file(&path, "read").expect("open for read");
    assert!(!eof(&mut handle).expect("eof before read"));
    assert_eq!(file_size(&mut handle).expect("size"), 11);
    assert!(handle.is_open());
    assert_eq!(read_string(&mut handle).expect("read"), "hello world");
    assert!(eof(&mut handle).expect("eof after read"));
    assert!(handle.is_open());
}

#[test]
fn append_adds_to_an_existing_file() {
    let temp = TempDir::new("append");
    let path = write_fixture(&temp, "log.txt", "one\n");

    let mut handle = open_file(&path, "append").expect("open for append");
    write_string(&mut handle, "two\n").expect("append");
    close_file(&mut handle);
    assert!(!handle.is_open());

    let mut handle = open_file(&path, "read").expect("open for read");
    assert_eq!(read_string(&mut handle).expect("read"), "one\ntwo\n");
}

#[test]
fn read_data_is_binary_safe_and_read_string_rejects_non_utf8() {
    let temp = TempDir::new("binary");
    let path = temp.text(&temp.file("blob.bin"));

    let bytes = [0x00_u8, 0xFF, 0xFE, 0x0A];
    let mut handle = open_file(&path, "write").expect("open for write");
    write_data(&mut handle, &bytes).expect("write bytes");
    close_file(&mut handle);

    let mut handle = open_file(&path, "read").expect("open for read");
    assert_eq!(read_data(&mut handle).expect("read bytes"), bytes);
    assert!(eof(&mut handle).expect("eof after read"));
    assert_eq!(file_size(&mut handle).expect("size"), 4);
}

#[test]
fn read_string_reports_invalid_utf8() {
    let temp = TempDir::new("utf8");
    let path = temp.text(&temp.file("bad.txt"));

    let mut handle = open_file(&path, "write").expect("open for write");
    write_data(&mut handle, &[0xFF, 0xFE]).expect("write");
    close_file(&mut handle);

    let mut handle = open_file(&path, "read").expect("open for read");
    assert!(matches!(
        read_string(&mut handle),
        Err(FileError::NotUtf8 { .. })
    ));
}

#[test]
fn eof_and_size_follow_the_read_position() {
    let temp = TempDir::new("eof");
    let empty = write_fixture(&temp, "empty.txt", "");

    let mut handle = open_file(&empty, "read").expect("open empty");
    assert!(eof(&mut handle).expect("empty is at eof"));
    assert_eq!(file_size(&mut handle).expect("empty size"), 0);
}

#[test]
fn copy_rename_and_delete() {
    let temp = TempDir::new("copy");
    let source = write_fixture(&temp, "source.txt", "payload");
    let copy = temp.text(&temp.file("copy.txt"));
    let moved = temp.text(&temp.file("moved.txt"));

    assert_eq!(copy_file(&source, &copy).expect("copy"), 7);
    rename_file(&copy, &moved).expect("rename");
    assert!(!Path::new(&copy).exists());
    assert!(Path::new(&moved).exists());

    delete_file(&moved).expect("delete");
    assert!(!Path::new(&moved).exists());
    assert!(matches!(
        delete_file(&moved),
        Err(FileError::Io {
            kind: std::io::ErrorKind::NotFound,
            ..
        })
    ));
}

#[test]
fn missing_file_open_for_read_is_a_not_found_error() {
    let temp = TempDir::new("missing");
    let missing = temp.text(&temp.file("nope.txt"));
    assert!(matches!(
        open_file(&missing, "read"),
        Err(FileError::Io {
            kind: std::io::ErrorKind::NotFound,
            ..
        })
    ));
}

#[test]
fn unknown_mode_is_rejected() {
    let temp = TempDir::new("mode");
    let path = temp.text(&temp.file("f.txt"));
    assert!(matches!(
        open_file(&path, "sideways"),
        Err(FileError::UnknownMode { .. })
    ));
}

#[test]
fn a_closed_handle_rejects_further_operations() {
    let temp = TempDir::new("closed");
    let path = write_fixture(&temp, "f.txt", "data");
    let mut handle = open_file(&path, "read").expect("open");
    close_file(&mut handle);

    assert!(matches!(
        read_string(&mut handle),
        Err(FileError::Closed { .. })
    ));
    assert!(matches!(eof(&mut handle), Err(FileError::Closed { .. })));
    assert!(matches!(
        file_size(&mut handle),
        Err(FileError::Closed { .. })
    ));
}

#[test]
fn create_examine_and_delete_directory() {
    let temp = TempDir::new("dir");
    let nested = temp.text(&temp.file("nested"));
    let document = temp.text(&temp.file("note.txt"));
    std::fs::write(&document, b"x").expect("write fixture");

    create_directory(&nested).expect("create directory");
    assert!(matches!(
        create_directory(&nested),
        Err(FileError::Io {
            kind: std::io::ErrorKind::AlreadyExists,
            ..
        })
    ));

    let entries = examine_directory(&temp.text(temp.path())).expect("examine");
    let names: Vec<&str> = entries.iter().map(|entry| entry.name.as_str()).collect();
    assert_eq!(names, vec!["nested", "note.txt"]);
    assert_eq!(entries[0].kind, EntryKind::Directory);
    assert_eq!(entries[0].kind.flag(), 1.0);
    assert_eq!(entries[1].kind, EntryKind::File);
    assert_eq!(entries[1].kind.flag(), 0.0);

    // An empty directory deletes, but a non-empty one is rejected.
    delete_directory(&nested).expect("empty directory deletes");
    create_directory(&nested).expect("recreate directory");
    std::fs::write(Path::new(&nested).join("child.txt"), b"x").expect("write child");
    assert!(delete_directory(&nested).is_err());
}

#[test]
fn examine_missing_directory_is_an_error() {
    let temp = TempDir::new("examine-missing");
    let missing = temp.text(&temp.file("void"));
    assert!(matches!(
        examine_directory(&missing),
        Err(FileError::Io {
            kind: std::io::ErrorKind::NotFound,
            ..
        })
    ));
}

#[test]
fn get_path_part_splits_a_windows_path() {
    let path = "C:\\folder\\note.txt";
    assert_eq!(get_path_part(path, "drive").expect("drive"), "C:");
    assert_eq!(get_path_part(path, "dir").expect("dir"), "C:\\folder\\");
    assert_eq!(get_path_part(path, "name").expect("name"), "note.txt");
    assert_eq!(get_path_part(path, "ext").expect("ext"), "txt");

    // A relative, extension-less path has an empty drive/dir/ext.
    assert_eq!(get_path_part("notes", "drive").expect("drive"), "");
    assert_eq!(get_path_part("notes", "dir").expect("dir"), "");
    assert_eq!(get_path_part("notes", "name").expect("name"), "notes");
    assert_eq!(get_path_part("notes", "ext").expect("ext"), "");

    assert!(matches!(
        get_path_part(path, "volume"),
        Err(FileError::UnknownPathPart { .. })
    ));
}
