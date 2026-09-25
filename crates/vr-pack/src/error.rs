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
}
