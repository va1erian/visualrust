//! PureBasic path splitting, e.g. `GetPathPart` and `GetFilePart`.

use std::path::{Component, Path};

use crate::error::FileError;

/// Returns one part of `path` (PureBasic `GetPathPart` and friends).
///
/// `part` is one of:
///
/// | part    | result for `C:\\folder\\note.txt` |
/// |---------|----------------------------------|
/// | `drive` | `C:`                             |
/// | `dir`   | `C:\\folder\\` (trailing separator) |
/// | `name`  | `note.txt`                       |
/// | `ext`   | `txt` (no leading dot)           |
///
/// The split is purely textual, so it works on a path that does not exist.
pub fn get_path_part(path: &str, part: &str) -> Result<String, FileError> {
    let path_ref = Path::new(path);
    match part {
        "drive" => Ok(drive(path_ref)),
        "dir" => Ok(directory(path_ref)),
        "name" => Ok(file_name(path_ref)),
        "ext" => Ok(extension(path_ref)),
        other => Err(FileError::UnknownPathPart {
            part: other.to_owned(),
        }),
    }
}

/// The `C:` volume prefix, or an empty string for a drive-less path.
fn drive(path: &Path) -> String {
    path.components()
        .find_map(|component| match component {
            Component::Prefix(prefix) => Some(prefix.as_os_str().to_string_lossy().into_owned()),
            _ => None,
        })
        .unwrap_or_default()
}

/// The directory portion, with a trailing separator like PureBasic.
fn directory(path: &Path) -> String {
    let Some(parent) = path.parent() else {
        return String::new();
    };
    if parent.as_os_str().is_empty() {
        return String::new();
    }
    let mut text = parent.to_string_lossy().into_owned();
    let separator = std::path::MAIN_SEPARATOR;
    if !text.ends_with(separator) && !text.ends_with('/') && !text.ends_with('\\') {
        text.push(separator);
    }
    text
}

/// The final component, or an empty string when there is none.
fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// The extension without its dot, or an empty string when there is none.
fn extension(path: &Path) -> String {
    path.extension()
        .map(|extension| extension.to_string_lossy().into_owned())
        .unwrap_or_default()
}
