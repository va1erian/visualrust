//! Typed failures for reading, writing and decoding a bundle.

use std::io;
use std::path::PathBuf;

/// Anything that can go wrong turning a [`Bundle`](crate::Bundle) into bytes or
/// back. Every variant is a value, never a panic, so callers can match on the
/// kind of corruption they hit.
#[derive(Debug, thiserror::Error)]
pub enum PackError {
    #[error("bundle i/o failed: {0}")]
    Io(#[from] io::Error),

    #[error("could not read `{}`: {source}", path.display())]
    ReadFile {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error(transparent)]
    Manifest(#[from] vr_core::manifest::ManifestError),

    /// The last bytes of the file are not a bundle footer, so there is nothing
    /// to read; the caller probably pointed at the unstubbed executable.
    #[error("no VisualRust bundle footer found at end of file")]
    MissingFooter,

    #[error("file is too small to hold a bundle footer")]
    TooShort,

    #[error("bundle footer is corrupt: {reason}")]
    CorruptFooter { reason: &'static str },

    #[error("unsupported bundle format version {found}")]
    UnsupportedVersion { found: u8 },

    #[error("bundle payload extends past the end of the file")]
    Truncated,

    /// The compressed bytes were changed, so they no longer match the digest
    /// recorded when the bundle was written.
    #[error("bundle integrity check failed: payload hash mismatch")]
    HashMismatch,

    #[error("could not decompress bundle payload: {0}")]
    Decompress(String),

    #[error("bundle payload is malformed: {0}")]
    Malformed(String),

    /// The caller's stub source has no runtime for this project type, so there
    /// is nothing to append a bundle to; guessing a path would ship a wrong app.
    #[error("no `{kind}` runtime stub is available for export")]
    StubUnavailable { kind: &'static str },

    /// A stub was named but is not a file on disk. Kept distinct from the
    /// unavailable case so the output pane can say which path was wrong.
    #[error("runtime stub `{}` does not exist or is not a file", path.display())]
    StubMissing { path: PathBuf },

    /// Export never overwrites; the caller must remove or rename the target.
    #[error("export target `{}` already exists", path.display())]
    OutputExists { path: PathBuf },

    /// Copying the stub to the target failed before the bundle was appended.
    #[error("could not copy stub `{from}` to `{to}`: {source}")]
    StubCopy {
        from: PathBuf,
        to: PathBuf,
        #[source]
        source: io::Error,
    },

    /// A `.ico` blob could not be split into `RT_ICON`/`RT_GROUP_ICON` entries.
    /// Best-effort patching treats this as a hard error because shipping a
    /// broken icon group is worse than shipping the stub's own icon.
    #[error("icon is not a valid .ico: {reason}")]
    InvalidIcon { reason: String },

    /// The manifest's version string is not a dot-separated numeric version.
    #[error("invalid version `{value}`: {reason}")]
    InvalidVersion { value: String, reason: &'static str },

    /// The Win32 resource APIs rejected the image (not a PE, output locked, ...).
    /// The input file is untouched: patching works on a staging copy.
    #[error("could not patch resources in `{}`: {message}", path.display())]
    ResourcePatch { path: PathBuf, message: String },

    /// Swapping the patched staging copy back over the target failed.
    #[error("could not replace `{}` with the patched copy: {source}", path.display())]
    ResourceCommit {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    /// The "build as a Rust project" target already holds a `Cargo.toml`;
    /// emitting would clobber a hand-edited build file, so the caller must pick
    /// a fresh directory rather than lose their dependencies.
    #[error("rust project target `{}` already contains a Cargo.toml", path.display())]
    RustProjectExists { path: PathBuf },

    /// Writing a generated project file failed (permissions, disk full, ...).
    #[error("could not write generated rust project file `{}`: {source}", path.display())]
    RustProjectWrite {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    /// `cargo build` was requested but no `cargo` is on `PATH`; the toolchain is
    /// the one prerequisite this mode cannot satisfy on its own.
    #[error("cargo build requested but `cargo` is not on PATH")]
    CargoUnavailable,

    /// Spawning cargo itself failed, usually a PATH or permission problem rather
    /// than a compile error.
    #[error("could not run `cargo build` in `{}`: {source}", root.display())]
    CargoSpawn {
        root: PathBuf,
        #[source]
        source: io::Error,
    },

    /// Cargo ran and the crate did not compile; the captured stderr carries the
    /// actual diagnostic for the output pane.
    #[error("`cargo build` failed in `{}`: {stderr}", root.display())]
    CargoBuildFailed { root: PathBuf, stderr: String },
}
