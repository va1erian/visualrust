//! Packaged app runtime: hosts a compiled Dyon program in a win32ui window.
//!
//! [`RuntimeApp`] is the bridge between win32ui's retained message loop and a
//! Dyon module: each incoming [`HandlerId`] is mapped to the name of a Dyon
//! function, which the runtime calls once per message. Dispatch is kept free of
//! win32ui types so it can be exercised headlessly.
//!
//! At startup the stub uses [`run_embedded`] to read the bundle appended to its
//! own executable, extract it to a scratch directory and run the manifest entry
//! through the host for the project `type`. [`run_embedded_from`] is the same
//! path over an explicit file, which keeps the loader testable without a real
//! packaged executable.

#![forbid(unsafe_code)]

mod app;
mod error;
mod loader;
mod runner;
mod tempdir;

pub use app::{DispatchError, HandlerId, RuntimeApp};
pub use error::RuntimeError;
pub use loader::{Extracted, ExtractedEntry, load_from_file};
pub use runner::{RunReport, run_embedded, run_embedded_from};
