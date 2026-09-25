//! Typed errors for the PureBasic-style string library.
//!
//! Dyon native functions can only fail with a `String`, so [`StringError`] is
//! rendered once at the boundary. Keeping the pure API typed means the string
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
