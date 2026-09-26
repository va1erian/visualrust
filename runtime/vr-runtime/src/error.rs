//! Typed failures for locating, extracting and running a bundled app.

use std::io;
use std::path::PathBuf;

use vr_pack::PackError;

/// Anything that can go wrong loading or running the app bundled into the
/// runtime executable. Every variant is a value, so the stub reports a failure
/// and exits non-zero instead of panicking.
#[derive(Debug, thiserror::Error)]
pub enum RuntimeError {
    /// The file's trailing bytes are not a bundle footer: it is a bare stub,
    /// not a packaged app. Distinguished from a corrupt bundle so the caller can
    /// tell "nothing to run" from "the payload is broken".
    #[error("no VisualRust bundle is appended to `{}`", .path.display())]
    NoBundle { path: PathBuf },

    /// A bundle footer was found but the payload could not be decoded.
    #[error(transparent)]
    Pack(#[from] PackError),

    /// The stub cannot find its own image, so it cannot find its payload. This
    /// also happens after the file is renamed or moved: `current_exe` is
    /// resolved at call time, not baked in.
    #[error("could not locate the running executable: {0}")]
    CurrentExe(#[source] io::Error),

    /// A scratch-directory operation failed.
    #[error("temp directory `{}` could not be {action}: {source}", .path.display())]
    TempDir {
        path: PathBuf,
        action: &'static str,
        #[source]
        source: io::Error,
    },

    /// An extracted blob could not be written into the scratch tree.
    #[error("could not extract `{name}` to `{}`: {source}", .path.display())]
    Extract {
        name: String,
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    /// A manifest-declared path tried to escape the extraction root. Validation
    /// should already reject this, but a bundle is untrusted input at run time.
    #[error("manifest path `{}` escapes the extraction root", .0.display())]
    UnsafePath(PathBuf),

    /// A bundled blob has no matching manifest reference, so its destination on
    /// disk is undefined.
    #[error("bundle contains a `{kind}` entry named `{name}` with no manifest reference")]
    UnmappedEntry { kind: &'static str, name: String },

    /// The extracted entry source could not be read back.
    #[error("could not read extracted entry `{}`: {source}", .path.display())]
    ReadEntry {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    /// The manifest names this app kind, but the stub does not host it yet.
    #[error("`{kind}` apps are not hosted by vr-runtime yet")]
    UnsupportedKind { kind: &'static str },

    /// The Dyon program failed to compile or run.
    #[error(transparent)]
    Dyon(#[from] vr_dyon::DyonError),
}
