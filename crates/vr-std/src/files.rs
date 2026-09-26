//! PureBasic-style file commands over `std::fs`.
//!
//! # Handles and modes
//!
//! [`open_file`] returns a [`FileHandle`] opened in one of three modes —
//! `"read"`, `"write"` (truncate or create) or `"append"` (create if missing).
//! The mode is a string rather than a numeric flag because Dyon has no enums and
//! a string reads at the call site. Handles are values: closing one drops the
//! underlying [`std::fs::File`], and any later command on it returns
//! [`FileError::Closed`] instead of touching a dangling descriptor.
//!
//! # Whole-file reads
//!
//! [`read_string`] and [`read_data`] read from the current position to the end
//! of the file. There is no seek command, so the position only moves forward;
//! [`eof`] reports whether it has reached the end and [`file_size`] reports the
//! total length. [`read_string`] additionally requires valid UTF-8, while
//! [`read_data`] is binary-safe and returns the raw bytes.
//!
//! # Paths
//!
//! Every command validates its path through the filesystem and reports the
//! failure as a typed [`FileError`]; nothing here panics. Directory commands
//! live in [`dir`](crate::files::dir) and the PureBasic path splitter in
//! [`path`](crate::files::path).

use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, Write};
use std::path::{Path, PathBuf};

use crate::error::FileError;

mod dir;
mod path;
#[cfg(test)]
mod tests;

pub use dir::{DirEntry, EntryKind, create_directory, delete_directory, examine_directory};
pub use path::get_path_part;

/// How [`open_file`] opens a handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileMode {
    /// Open an existing file for reading.
    Read,
    /// Create or truncate a file for writing.
    Write,
    /// Create a file if missing and write at the end.
    Append,
}

impl FileMode {
    /// Parses the Dyon-facing mode string.
    pub fn parse(mode: &str) -> Result<Self, FileError> {
        match mode {
            "read" => Ok(Self::Read),
            "write" => Ok(Self::Write),
            "append" => Ok(Self::Append),
            other => Err(FileError::UnknownMode {
                mode: other.to_owned(),
            }),
        }
    }

    /// The `OpenOptions` for this mode.
    fn options(self) -> OpenOptions {
        let mut options = OpenOptions::new();
        match self {
            Self::Read => {
                options.read(true);
            }
            Self::Write => {
                options.write(true).create(true).truncate(true);
            }
            Self::Append => {
                options.append(true).create(true);
            }
        }
        options
    }
}

/// An open file, its path and the mode it was opened with.
pub struct FileHandle {
    path: PathBuf,
    mode: FileMode,
    file: Option<File>,
}

impl FileHandle {
    /// Opens `path` in `mode`.
    pub fn open(path: impl AsRef<Path>, mode: FileMode) -> Result<Self, FileError> {
        let path = path.as_ref();
        let file = mode
            .options()
            .open(path)
            .map_err(|source| io_error("open_file", path, &source))?;
        Ok(Self {
            path: path.to_path_buf(),
            mode,
            file: Some(file),
        })
    }

    /// The path the handle was opened on.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The mode the handle was opened with.
    pub fn mode(&self) -> FileMode {
        self.mode
    }

    /// Whether the handle is still open.
    pub fn is_open(&self) -> bool {
        self.file.is_some()
    }

    /// Borrows the underlying file, or reports that the handle was closed.
    ///
    /// Keeping the file in an `Option` is what lets `close_file` be a command
    /// rather than a consumed Rust value: the handle object lives on in Dyon,
    /// but every later operation fails cleanly.
    fn file_mut(&mut self, function: &'static str) -> Result<&mut File, FileError> {
        self.file.as_mut().ok_or(FileError::Closed { function })
    }

    /// Closes the handle. A second close is a no-op.
    pub fn close(&mut self) {
        self.file = None;
    }
}

/// Builds a typed I/O error for a path.
pub(crate) fn io_error(function: &'static str, path: &Path, source: &std::io::Error) -> FileError {
    FileError::Io {
        function,
        path: path.display().to_string(),
        kind: source.kind(),
        message: source.to_string(),
    }
}

/// Opens `path` in the mode named by the string `mode` (PureBasic `OpenFile`).
pub fn open_file(path: &str, mode: &str) -> Result<FileHandle, FileError> {
    FileHandle::open(path, FileMode::parse(mode)?)
}

/// Reads the rest of the file as UTF-8 text (PureBasic `ReadString`).
pub fn read_string(handle: &mut FileHandle) -> Result<String, FileError> {
    let bytes = read_data(handle)?;
    String::from_utf8(bytes).map_err(|_| FileError::NotUtf8 {
        function: "read_string",
    })
}

/// Writes `text` at the current position (PureBasic `WriteString`).
pub fn write_string(handle: &mut FileHandle, text: &str) -> Result<(), FileError> {
    write_data(handle, text.as_bytes())
}

/// Reads the rest of the file as raw bytes (PureBasic `ReadData`).
pub fn read_data(handle: &mut FileHandle) -> Result<Vec<u8>, FileError> {
    let path = handle.path.clone();
    let file = handle.file_mut("read_data")?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)
        .map_err(|source| io_error("read_data", &path, &source))?;
    Ok(bytes)
}

/// Writes raw `bytes` at the current position and flushes (PureBasic
/// `WriteData`).
///
/// Flushing means a script that writes then immediately reopens the file sees
/// the bytes, which is the behaviour a test or an export step expects.
pub fn write_data(handle: &mut FileHandle, bytes: &[u8]) -> Result<(), FileError> {
    let path = handle.path.clone();
    let file = handle.file_mut("write_data")?;
    file.write_all(bytes)
        .and_then(|()| file.flush())
        .map_err(|source| io_error("write_data", &path, &source))
}

/// Closes `handle` (PureBasic `CloseFile`).
pub fn close_file(handle: &mut FileHandle) {
    handle.close();
}

/// Whether the read position is at or past the end of the file (PureBasic
/// `Eof`).
pub fn eof(handle: &mut FileHandle) -> Result<bool, FileError> {
    let path = handle.path.clone();
    let file = handle.file_mut("eof")?;
    let position = file
        .stream_position()
        .map_err(|source| io_error("eof", &path, &source))?;
    let length = file
        .metadata()
        .map_err(|source| io_error("eof", &path, &source))?
        .len();
    Ok(position >= length)
}

/// The total size of the file in bytes (PureBasic `Lof`).
pub fn file_size(handle: &mut FileHandle) -> Result<u64, FileError> {
    let path = handle.path.clone();
    let file = handle.file_mut("file_size")?;
    let length = file
        .metadata()
        .map_err(|source| io_error("file_size", &path, &source))?
        .len();
    Ok(length)
}

/// Deletes the file at `path` (PureBasic `DeleteFile`).
pub fn delete_file(path: &str) -> Result<(), FileError> {
    let path_ref = Path::new(path);
    std::fs::remove_file(path_ref).map_err(|source| io_error("delete_file", path_ref, &source))
}

/// Copies `from` to `to`, returning the number of bytes copied (PureBasic
/// `CopyFile`).
pub fn copy_file(from: &str, to: &str) -> Result<u64, FileError> {
    let from_ref = Path::new(from);
    std::fs::copy(from_ref, to).map_err(|source| io_error("copy_file", from_ref, &source))
}

/// Renames or moves `from` to `to` (PureBasic `RenameFile`).
pub fn rename_file(from: &str, to: &str) -> Result<(), FileError> {
    let from_ref = Path::new(from);
    std::fs::rename(from_ref, to).map_err(|source| io_error("rename_file", from_ref, &source))
}
