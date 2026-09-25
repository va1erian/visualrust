//! PureBasic-inspired standard library.
//!
//! `vr-std` owns both the safe Rust implementations of the library commands
//! and the Dyon natives that expose them. It depends only on `dyon` and
//! `thiserror`: the runtime assembly passes [`register`] into
//! `DyonRuntime::from_source_with`, so `vr-dyon` never depends on this crate.
//!
//! # Strings
//!
//! [`strings`] is the pure, typed API. Its positions are **1-based** and count
//! `char`s, matching PureBasic; invalid arguments produce a typed
//! [`StringError`] instead of a panic. The same commands are available to Dyon
//! once [`register`] has run:
//!
//! ```dyon
//! fn main() {
//!     println(ucase(mid("VisualRust", 7, 4))) // RUST
//! }
//! ```

#![forbid(unsafe_code)]

pub mod error;
pub mod strings;

mod native;

pub use error::StringError;
pub use native::register;
