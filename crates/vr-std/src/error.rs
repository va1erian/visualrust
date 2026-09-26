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

/// A math argument outside a function's real domain.
///
/// [`crate::math`] rejects `sqrt`/`ln` of a negative and `asin`/`acos` outside
/// `[-1, 1]` instead of silently returning an IEEE NaN, so a script author gets
/// the same typed failure they get from the string and encoding commands.
#[derive(Debug, Error, Clone, PartialEq)]
pub enum MathError {
    /// The argument has no real result for this function.
    #[error("{function}: {value} is outside the function's domain")]
    Domain {
        /// The Dyon command that rejected the argument.
        function: &'static str,
        /// The rejected value.
        value: f64,
    },
}

/// A rejected date/time argument, format string or parsed field.
///
/// [`crate::datetime`] formats and parses UTC instants from an explicit value,
/// so its failures are about the text and the instant rather than about the
/// clock: an unknown `%` specifier, a format that does not match the text, or a
/// field outside its real range. The timestamp bound exists because the civil
/// calendar conversions are only exact for the documented `1..=9999` year
/// range.
#[derive(Debug, Error, Clone, PartialEq)]
pub enum DateTimeError {
    /// The format string ended with a `%` that introduced no specifier.
    #[error("{function}: the format string ends with a bare '%'")]
    DanglingPercent {
        /// The Dyon command that rejected the argument.
        function: &'static str,
    },
    /// The format string used a `%` specifier the library does not know.
    #[error("{function}: '{specifier}' is not a known format specifier")]
    UnknownSpecifier {
        /// The Dyon command that rejected the argument.
        function: &'static str,
        /// The unrecognised specifier character.
        specifier: char,
    },
    /// The text ran out or disagreed with the format at `position`.
    #[error("{function}: the text does not match the format at character {position}")]
    ParseMismatch {
        /// The Dyon command that rejected the argument.
        function: &'static str,
        /// Character offset where the match failed.
        position: usize,
    },
    /// The format omitted a date field that parsing requires.
    #[error("{function}: the format is missing a '{field}' field")]
    MissingField {
        /// The Dyon command that rejected the argument.
        function: &'static str,
        /// The missing field name.
        field: &'static str,
    },
    /// A parsed field was outside its real range.
    #[error("{function}: {field} value {value} is out of range")]
    FieldOutOfRange {
        /// The Dyon command that rejected the argument.
        function: &'static str,
        /// The rejected field name.
        field: &'static str,
        /// The rejected value.
        value: i64,
    },
    /// An instant cannot be represented by the supported calendar range.
    #[error("{function}: timestamp {value} is outside the supported range")]
    TimestampOutOfRange {
        /// The Dyon command that rejected the argument.
        function: &'static str,
        /// The rejected whole-second UTC timestamp.
        value: i64,
    },
    /// A numeric argument was not the expected kind of count.
    #[error("{function}: {value} is not {expected}")]
    InvalidArgument {
        /// The Dyon command that rejected the argument.
        function: &'static str,
        /// The rejected value.
        value: f64,
        /// What the argument should have been.
        expected: &'static str,
    },
}

/// A rejected file, directory or console argument.
///
/// The pure file API turns every `std::io` failure into a typed [`FileError`]
/// instead of handing the raw error on, so one message renders at the Dyon
/// boundary and tests can match the cause (`NotFound`, `AlreadyExists`, ...)
/// without depending on the OS error string. `message` carries that string for
/// the human reading the script output.
#[derive(Debug, Error, Clone, PartialEq)]
pub enum FileError {
    /// The filesystem rejected an operation.
    #[error("{function}: `{path}`: {message}")]
    Io {
        /// The Dyon command that failed.
        function: &'static str,
        /// The path the command acted on, or the stream name for console I/O.
        path: String,
        /// The recognisable category from `std::io`.
        kind: std::io::ErrorKind,
        /// The OS description of the failure.
        message: String,
    },
    /// The mode passed to `open_file` was not `read`, `write` or `append`.
    #[error("open_file: unknown mode `{mode}` (expected `read`, `write` or `append`)")]
    UnknownMode {
        /// The rejected mode string.
        mode: String,
    },
    /// An operation ran on a handle that had already been closed.
    #[error("{function}: the file handle is closed")]
    Closed {
        /// The Dyon command that failed.
        function: &'static str,
    },
    /// `read_string` found bytes that are not valid UTF-8.
    #[error("{function}: the file contents are not valid UTF-8")]
    NotUtf8 {
        /// The Dyon command that failed.
        function: &'static str,
    },
    /// A Dyon value that should have been a file handle was some other type.
    #[error("value is not a file handle")]
    NotAHandle,
    /// A previous panic left a handle's mutex unusable.
    #[error("the file handle was poisoned by an earlier panic")]
    Poisoned,
    /// `get_path_part` was asked for a part it does not define.
    #[error("get_path_part: `{part}` is not a known part (expected drive, dir, name or ext)")]
    UnknownPathPart {
        /// The rejected part name.
        part: String,
    },
    /// `write_data` was given something other than an array.
    #[error("expected an array of bytes 0-255, got {0}")]
    NotAnArray(String),
    /// A `write_data` element was not a whole number in `0..=255`.
    #[error("a byte must be a whole number 0-255, got `{0}`")]
    InvalidByteElement(String),
}
