//! Unit tests for the CLI-argument resolution, kept beside (but out of) the
//! builder so `mod.rs` stays inside the file-length limit.

use super::*;

#[test]
fn a_directory_argument_is_the_project_root() {
    let dir = std::env::temp_dir();
    let (start, file) = split_argument(Some(dir.clone()), Path::new("."));
    assert_eq!(start, dir);
    assert!(file.is_none());
}

#[test]
fn a_file_argument_opens_it_and_uses_its_parent() {
    let (start, file) = split_argument(Some(PathBuf::from("proj/main.dyon")), Path::new("."));
    assert_eq!(start, PathBuf::from("proj"));
    assert_eq!(file, Some(PathBuf::from("proj/main.dyon")));
}
