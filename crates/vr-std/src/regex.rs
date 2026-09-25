//! Regular-expression match and replace over UTF-8 text.
//!
//! These wrap the [`regex`] crate. A pattern that fails to compile becomes a
//! typed [`RegexError::InvalidPattern`] instead of a panic, so a bad script
//! pattern surfaces as a normal Dyon runtime error.
//!
//! Replacements use the `regex` crate's own `$1`/`${name}` capture syntax. That
//! is deliberate: re-inventing PureBasic's numbered back-references would only
//! add a second, subtly different dialect.

use regex::Regex;

use crate::error::RegexError;

/// Compiles `pattern`, mapping the compiler error to a typed value.
fn compile(pattern: &str, function: &'static str) -> Result<Regex, RegexError> {
    Regex::new(pattern).map_err(|error| RegexError::InvalidPattern {
        function,
        message: error.to_string(),
    })
}

/// True when `pattern` matches anywhere in `text` (unanchored search).
pub fn regex_match(pattern: &str, text: &str) -> Result<bool, RegexError> {
    Ok(compile(pattern, "regex_match")?.is_match(text))
}

/// Replaces every non-overlapping match of `pattern` in `text`.
pub fn regex_replace(pattern: &str, replacement: &str, text: &str) -> Result<String, RegexError> {
    Ok(compile(pattern, "regex_replace")?
        .replace_all(text, replacement)
        .into_owned())
}

#[cfg(test)]
mod tests;
