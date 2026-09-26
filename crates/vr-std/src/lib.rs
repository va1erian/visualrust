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
//!
//! # Math
//!
//! [`math`] covers the PureBasic numeric commands: rounding, powers and roots,
//! trigonometry, logarithms and a small deterministic PRNG. Dyon's only number
//! type is `f64`, so every command takes and returns a float; the rounding
//! table and the NaN/domain rules are documented on the module.
//!
//! ```dyon
//! fn main() {
//!     println(sqrt(pow(3, 2) + pow(4, 2))) // 5
//! }
//! ```
//!
//! # Date and time
//!
//! [`datetime`] covers the PureBasic time commands: [`date`](datetime::date),
//! [`time`](datetime::time), [`elapsed_milliseconds`](datetime::elapsed_milliseconds),
//! [`format_date`](datetime::format_date), [`parse_date`](datetime::parse_date)
//! and [`delay`](datetime::delay). Every command is **UTC** — `std` cannot read
//! the local time zone without platform code, so the library documents one
//! default instead of a half-supported local mode. Wall-clock and monotonic
//! reads go through a [`Clock`], which tests freeze so assertions never depend
//! on the current time.
//!
//! ```dyon
//! fn main() {
//!     println(format_date(0, "%Y-%m-%d %H:%M:%S")) // 1970-01-01 00:00:00
//! }
//! ```

#![forbid(unsafe_code)]

pub mod datetime;
pub mod encoding;
pub mod error;
pub mod hash;
pub mod math;
pub mod regex;
pub mod strings;

mod native;

pub use datetime::{Clock, SystemClock};
pub use error::{DateTimeError, EncodingError, HashError, MathError, RegexError, StringError};
pub use native::register;
