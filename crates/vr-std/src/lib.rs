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
//!
//! # Encoding, regex and hashing
//!
//! [`encoding`] holds base64 and URL percent-encoding, [`regex`] match and
//! replace, and [`hash`] the md5/sha1/sha256 digests plus HMAC-SHA256. They
//! follow the same rule as the string library: typed `thiserror` errors, no
//! panics, and every fallible command surfaces as a Dyon runtime error.

#![forbid(unsafe_code)]

pub mod encoding;
pub mod error;
pub mod hash;
pub mod regex;
pub mod strings;

mod native;

pub use error::{EncodingError, HashError, RegexError, StringError};
pub use native::register;
