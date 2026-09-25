//! Typed errors for the PureBasic-style standard library.
//!
//! Dyon native functions can only fail with a `String`, so each error is
//! rendered once at the boundary. Keeping the pure API typed means the
//! functions stay panic-free and their failures can be asserted in tests.

use thiserror::Error;

/// A rejected string argument.
///
/// The `function` field names the Dyon command (`"mid"`, `"lset"`, ...) so the
/// message reads sensibly no matter which wrapper surfaced it.
#[derive(Debug, Error, Clone, PartialEq)]
pub enum StringError {
    /// A length-like argument was negative.
    #[error("{function}: length must not be negative (got {value})")]
    NegativeLength {
        /// The Dyon command that rejected the argument.
        function: &'static str,
        /// The rejected value.
        value: i64,
    },
    /// A 1-based position named a character that does not exist.
    #[error("{function}: index {index} is out of range (the string has {count} characters)")]
    IndexOutOfRange {
        /// The Dyon command that rejected the argument.
        function: &'static str,
        /// The rejected 1-based position.
        index: i64,
        /// The number of characters in the string.
        count: usize,
    },
    /// An insertion position fell outside `1..=len + 1`.
    #[error("{function}: position {position} is out of range (valid positions are 1..={max})")]
    PositionOutOfRange {
        /// The Dyon command that rejected the argument.
        function: &'static str,
        /// The rejected 1-based position.
        position: i64,
        /// The largest accepted position.
        max: usize,
    },
    /// A length-like argument exceeded the crate's allocation guard.
    #[error("{function}: length {value} is larger than the {limit} limit")]
    LengthTooLarge {
        /// The Dyon command that rejected the argument.
        function: &'static str,
        /// The rejected value.
        value: i64,
        /// The inclusive upper bound.
        limit: usize,
    },
    /// A numeric argument was not a finite integer.
    #[error("{function}: {value} is not a finite integer")]
    NotAnInteger {
        /// The Dyon command that rejected the argument.
        function: &'static str,
        /// The rejected value.
        value: f64,
    },
    /// A numeric argument was not a non-negative integer.
    #[error("{function}: {value} must be a non-negative integer")]
    NotNonNegativeInteger {
        /// The Dyon command that rejected the argument.
        function: &'static str,
        /// The rejected value.
        value: f64,
    },
    /// A field index was below 1.
    #[error("{function}: field index must be at least 1 (got {index})")]
    InvalidFieldIndex {
        /// The Dyon command that rejected the argument.
        function: &'static str,
        /// The rejected 1-based field index.
        index: i64,
    },
}

/// A rejected base64 or percent-encoded argument.
///
/// `function` names the Dyon command so the message reads sensibly no matter
/// which wrapper surfaced it. The decoder failures deliberately drop the
/// underlying library error: it carries a byte offset into the encoded text,
/// which is less useful to a script author than the command name.
#[derive(Debug, Error, Clone, PartialEq)]
pub enum EncodingError {
    /// The text was not a well-formed base64 string.
    #[error("{function}: the text is not valid base64")]
    InvalidBase64 {
        /// The Dyon command that rejected the argument.
        function: &'static str,
    },
    /// Decoded bytes were not valid UTF-8, so Dyon cannot hold them.
    #[error("{function}: the decoded bytes are not valid UTF-8")]
    NotUtf8 {
        /// The Dyon command that rejected the argument.
        function: &'static str,
    },
    /// A `%` was not followed by two hexadecimal digits.
    #[error("{function}: invalid percent escape at byte {position}")]
    InvalidPercentEscape {
        /// The Dyon command that rejected the argument.
        function: &'static str,
        /// Byte offset of the offending `%`.
        position: usize,
    },
}

/// A rejected regular expression argument.
///
/// Compilation is the only fallible step; the underlying `regex` error is kept
/// as text because it is not `Clone` and cannot be embedded in the enum.
#[derive(Debug, Error, Clone, PartialEq)]
pub enum RegexError {
    /// The pattern did not compile.
    #[error("{function}: invalid regular expression: {message}")]
    InvalidPattern {
        /// The Dyon command that rejected the argument.
        function: &'static str,
        /// The compiler's description of the problem.
        message: String,
    },
}

/// A rejected hashing argument.
#[derive(Debug, Error, Clone, PartialEq)]
pub enum HashError {
    /// The key length was unacceptable to the HMAC construction.
    ///
    /// SHA-256 HMAC accepts any key length, so this is defensive; it exists so
    /// the key path returns a typed error instead of unwrapping a `Result`.
    #[error("hmac_sha256: the key could not be used")]
    InvalidKey,
}
