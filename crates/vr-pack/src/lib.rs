//! Packaging: the compressed project payload appended to a runtime stub.
//!
//! An exported app is a prebuilt `vr-runtime.exe` with a [`Bundle`] appended at
//! EOF. The bundle holds the validated [`Manifest`](vr_core::manifest::Manifest)
//! plus every named blob the app needs: Dyon sources, precompiled `.vrform`
//! forms, assets, an optional SQLite database and native extension DLLs.
//!
//! # Layout
//!
//! The appended bytes are a deflate-compressed, length-prefixed payload followed
//! by a fixed-size footer:
//!
//! ```text
//! [ deflated payload ][ footer: magic | version | compressor |
//!                       payload_offset | payload_len | raw_len | sha256 ]
//! ```
//!
//! The footer lives at EOF, so a reader seeks to the last [`FOOTER_LEN`](format::FOOTER_LEN)
//! bytes, parses it, and loads only the payload range. Appending therefore
//! never rewrites the PE header, and the runtime (#51) can hand the bundle back
//! without disturbing the stub. The SHA-256 covers the *compressed* payload, so
//! corruption is caught before decompression; a byte flip or truncation yields
//! a typed [`PackError`], never a panic.
//!
//! [`export`] assembles an output executable by copying the stub its manifest
//! type selects and appending the bundle. Icon/version patching is #53 and the
//! "build as a Rust project" mode is #70; neither is done here.

#![forbid(unsafe_code)]

mod bundle;
mod codec;
mod error;
mod export;
pub mod format;

pub use bundle::{Bundle, BundleEntry, EntryKind};
pub use error::PackError;
pub use export::{StubSource, Stubs, export, export_with_stub};
