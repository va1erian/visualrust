//! PureBasic-inspired standard library.
//!
//! `vr-std` owns both the safe Rust implementations of the library commands
//! and the Dyon natives that expose them. It depends only on `dyon` and
//! `thiserror`: the runtime assembly passes [`register`] into
//! `DyonRuntime::from_source_with`, so `vr-dyon` never depends on this crate.
//!
//! # Naming
//!
//! Dyon resolves a function call by name only, so a native registered under a
//! built-in's name shadows it for every later script. PureBasic commands that
//! would collide therefore register under a prefixed name — `str_len`,
//! `str_trim`, `num_str` and the `num_*` math commands — while names Dyon does
//! not use keep the PureBasic spelling.
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
//!     println(num_sqrt(num_pow(3, 2) + num_pow(4, 2))) // 5
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
//!
//! # Files and directories
//!
//! [`files`] covers the PureBasic file commands. Handles are Dyon custom
//! objects opened in `"read"`, `"write"` or `"append"` mode; reads are whole
//! file (or whole remainder), [`files::read_data`]/[`files::write_data`] are
//! binary-safe byte arrays, and the directory commands create, delete and list
//! entries. Path validation always produces a typed [`FileError`] rather than a
//! panic, and nothing in a test touches real user data.
//!
//! ```dyon
//! fn main() {
//!     f := open_file("hello.txt", "write")
//!     write_string(f, "hi")
//!     close_file(f)
//!     println(get_path_part("C:\\tmp\\hello.txt", "name")) // hello.txt
//! }
//! ```
//!
//! # Console
//!
//! [`console`] adds only PureBasic's `Input` (`input`), which prompts before
//! reading a line. Dyon's `stdio` feature already supplies `print`, `println`,
//! `eprint`, `eprintln` and `read_line`, so those are left alone rather than
//! shadowed.

#![forbid(unsafe_code)]

pub mod console;
pub mod datetime;
pub mod encoding;
pub mod error;
pub mod files;
pub mod hash;
pub mod math;
pub mod regex;
pub mod strings;

mod native;

pub use datetime::{Clock, SystemClock};
pub use error::{
    DateTimeError, EncodingError, FileError, HashError, MathError, RegexError, StringError,
};
pub use native::register;
