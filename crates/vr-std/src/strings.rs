//! PureBasic-style string operations over Unicode scalar values.
//!
//! # Indexing
//!
//! Every position argument and the result of [`find_string`] are **1-based**,
//! matching the PureBasic commands (`Left`, `Mid`, `FindString`,
//! `StringField`). Positions count `char`s (Unicode scalar values), never
//! bytes, so `len("héllo") == 5`.
//!
//! A 1-based position must name an existing character: [`mid`],
//! [`remove_string`] and [`string_field`] reject an out-of-range position with a
//! typed [`StringError`] instead of silently truncating. [`insert_string`]
//! additionally accepts `len + 1` to append. [`left`]/[`right`] and the length
//! inside [`mid`]/[`remove_string`] clamp the way PureBasic clamps them, and
//! [`string_field`] returns an empty string for a field that simply does not
//! exist because fields are often optional.

use crate::error::StringError;

/// Upper bound for a string we are willing to build by repetition or padding.
pub const MAX_LENGTH: usize = 10_000_000;

/// Upper bound for a fixed-decimal precision request.
pub const MAX_PRECISION: usize = 1_000;

/// Upper bound for a hexadecimal/binary minimum width.
pub const MAX_DIGITS: usize = 1_024;

/// Converts a Dyon number into an exact 1-based index or length.
///
/// Dyon only has `f64`, so an index arrives here as a float; anything with a
/// fractional part or a non-finite value is not a valid position.
pub fn to_index(value: f64, function: &'static str) -> Result<i64, StringError> {
    if value.is_finite() && value.fract() == 0.0 {
        Ok(value as i64)
    } else {
        Err(StringError::NotAnInteger { function, value })
    }
}

/// The characters of `text`, so slicing cannot split a multi-byte code point.
fn chars(text: &str) -> Vec<char> {
    text.chars().collect()
}

/// Validates a non-negative, bounded length argument.
fn checked_length(value: i64, function: &'static str, limit: usize) -> Result<usize, StringError> {
    if value < 0 {
        return Err(StringError::NegativeLength { function, value });
    }
    let length = value as usize;
    if length > limit {
        return Err(StringError::LengthTooLarge {
            function,
            value,
            limit,
        });
    }
    Ok(length)
}

/// Maps a 1-based position onto a slice start, rejecting positions outside
/// `1..=count`.
fn positional_index(
    position: i64,
    count: usize,
    function: &'static str,
) -> Result<usize, StringError> {
    if position < 1 || position as usize > count {
        Err(StringError::IndexOutOfRange {
            function,
            index: position,
            count,
        })
    } else {
        Ok((position - 1) as usize)
    }
}

/// The first `count` characters of `text` (PureBasic `Left`).
pub fn left(text: &str, count: i64) -> Result<String, StringError> {
    let count = checked_length(count, "left", MAX_LENGTH)?;
    Ok(text.chars().take(count).collect())
}

/// The last `count` characters of `text` (PureBasic `Right`).
pub fn right(text: &str, count: i64) -> Result<String, StringError> {
    let count = checked_length(count, "right", MAX_LENGTH)?;
    let all = chars(text);
    let start = all.len().saturating_sub(count);
    Ok(all[start..].iter().collect())
}

/// `length` characters of `text` starting at the 1-based `start`
/// (PureBasic `Mid`).
pub fn mid(text: &str, start: i64, length: i64) -> Result<String, StringError> {
    let all = chars(text);
    let start = positional_index(start, all.len(), "mid")?;
    let length = checked_length(length, "mid", MAX_LENGTH)?;
    Ok(all[start..].iter().take(length).collect())
}

/// The number of characters in `text` (PureBasic `Len`).
pub fn len(text: &str) -> usize {
    text.chars().count()
}

/// `text` upper-cased, using Unicode-aware mapping (PureBasic `UCase`).
///
/// Mapping can change the character count (`"ß".to_uppercase()` is `"SS"`),
/// which matches the Unicode rules rather than a byte-wise transform.
pub fn ucase(text: &str) -> String {
    text.to_uppercase()
}

/// `text` lower-cased, using Unicode-aware mapping (PureBasic `LCase`).
pub fn lcase(text: &str) -> String {
    text.to_lowercase()
}

/// The 1-based character position of the first `needle` in `text`, or `0` when
/// it is absent (PureBasic `FindString`).
///
/// An empty `needle` is reported at position 1, the position of the empty match
/// at the start.
pub fn find_string(text: &str, needle: &str) -> i64 {
    if needle.is_empty() {
        return 1;
    }
    match text.find(needle) {
        Some(byte) => (text[..byte].chars().count() + 1) as i64,
        None => 0,
    }
}

/// Replaces every non-overlapping `search` in `text` with `replacement`
/// (PureBasic `ReplaceString`).
///
/// An empty `search` is returned unchanged: Rust's own `replace` would wedge
/// the replacement between every character, which is never what PureBasic
/// means.
pub fn replace_string(text: &str, search: &str, replacement: &str) -> String {
    if search.is_empty() {
        return text.to_owned();
    }
    text.replace(search, replacement)
}

/// `text` without leading and trailing whitespace (PureBasic `Trim`).
///
/// Whitespace is the Unicode `char::is_whitespace` class, not just ASCII.
pub fn trim(text: &str) -> String {
    text.trim().to_owned()
}

/// `text` without leading whitespace (PureBasic `Trim` mode 1).
pub fn ltrim(text: &str) -> String {
    text.trim_start().to_owned()
}

/// `text` without trailing whitespace (PureBasic `Trim` mode 2).
pub fn rtrim(text: &str) -> String {
    text.trim_end().to_owned()
}

/// The number of non-overlapping `needle` occurrences in `text`
/// (PureBasic `CountString`).
pub fn count_string(text: &str, needle: &str) -> usize {
    if needle.is_empty() {
        return 0;
    }
    text.match_indices(needle).count()
}

/// Inserts `insert` before the 1-based `position` of `text`
/// (PureBasic `InsertString`).
pub fn insert_string(text: &str, position: i64, insert: &str) -> Result<String, StringError> {
    let all = chars(text);
    let max = all.len() + 1;
    if position < 1 || position as usize > max {
        return Err(StringError::PositionOutOfRange {
            function: "insert_string",
            position,
            max,
        });
    }
    let at = (position - 1) as usize;
    let mut result: String = all[..at].iter().collect();
    result.push_str(insert);
    result.extend(&all[at..]);
    Ok(result)
}

/// Removes `length` characters from `text` starting at the 1-based `start`
/// (PureBasic `RemoveString`).
pub fn remove_string(text: &str, start: i64, length: i64) -> Result<String, StringError> {
    let all = chars(text);
    let start = positional_index(start, all.len(), "remove_string")?;
    let length = checked_length(length, "remove_string", MAX_LENGTH)?;
    let end = (start + length).min(all.len());
    let mut result: String = all[..start].iter().collect();
    result.extend(&all[end..]);
    Ok(result)
}

/// The 1-based `index`-th field of `text`, split on `delimiter`
/// (PureBasic `StringField`).
///
/// A field beyond the last one is empty rather than an error: field lists are
/// commonly ragged, and PureBasic returns `""` there. An empty `delimiter`
/// cannot split, so the whole `text` is the single field.
pub fn string_field(text: &str, index: i64, delimiter: &str) -> Result<String, StringError> {
    if index < 1 {
        return Err(StringError::InvalidFieldIndex {
            function: "string_field",
            index,
        });
    }
    if delimiter.is_empty() {
        return Ok(if index == 1 {
            text.to_owned()
        } else {
            String::new()
        });
    }
    Ok(text
        .split(delimiter)
        .nth((index - 1) as usize)
        .unwrap_or("")
        .to_owned())
}

/// `count` spaces (PureBasic `Space`).
pub fn space(count: i64) -> Result<String, StringError> {
    let count = checked_length(count, "space", MAX_LENGTH)?;
    Ok(" ".repeat(count))
}

/// Reads the longest numeric prefix of `text` as a float
/// (PureBasic `Val`).
///
/// Leading whitespace is ignored and `0.0` is returned when no number is
/// present, matching PureBasic's forgiving `Val`.
pub fn val(text: &str) -> f64 {
    let trimmed = text.trim_start();
    let mut end = trimmed.len();
    loop {
        if let Ok(value) = trimmed[..end].parse::<f64>() {
            return value;
        }
        match trimmed[..end].char_indices().next_back() {
            Some((index, _)) => end = index,
            None => return 0.0,
        }
    }
}

/// Renders `value` for display (PureBasic `Str`).
///
/// Unlike PureBasic, which rounds to an integer, this keeps the fractional
/// part: Dyon's only number type is `f64`, so dropping it would lose data.
/// Whole numbers lose the trailing `.0`.
pub fn number_to_string(value: f64) -> String {
    if value.is_finite() && value.fract() == 0.0 {
        return format!("{value:.0}");
    }
    format!("{value}")
}

/// Validates a non-negative integer argument for `hex`/`bin`.
fn non_negative(value: f64, function: &'static str) -> Result<u64, StringError> {
    if value.is_finite() && value.fract() == 0.0 && value >= 0.0 && value <= u64::MAX as f64 {
        Ok(value as u64)
    } else {
        Err(StringError::NotNonNegativeInteger { function, value })
    }
}

/// `value` in upper-case hexadecimal, zero-padded to at least `digits`
/// (PureBasic `Hex`).
pub fn hex(value: f64, digits: i64) -> Result<String, StringError> {
    let digits = checked_length(digits, "hex", MAX_DIGITS)?;
    let number = non_negative(value, "hex")?;
    let mut out = format!("{number:X}");
    while out.len() < digits {
        out.insert(0, '0');
    }
    Ok(out)
}

/// `value` in binary, zero-padded to at least `digits` (PureBasic `Bin`).
pub fn bin(value: f64, digits: i64) -> Result<String, StringError> {
    let digits = checked_length(digits, "bin", MAX_DIGITS)?;
    let number = non_negative(value, "bin")?;
    let mut out = format!("{number:b}");
    while out.len() < digits {
        out.insert(0, '0');
    }
    Ok(out)
}

/// `value` with exactly `digits` fractional digits (PureBasic `FormatNumber`).
pub fn format(value: f64, digits: i64) -> Result<String, StringError> {
    let digits = checked_length(digits, "format", MAX_PRECISION)?;
    Ok(format!("{value:.digits$}"))
}

/// Left-aligns `text` in a field of `length`, padding on the right with the
/// first character of `pad` (PureBasic `LSet`).
///
/// A `text` longer than `length` keeps its first `length` characters, and an
/// empty `pad` pads with spaces.
pub fn lset(text: &str, length: i64, pad: &str) -> Result<String, StringError> {
    let width = checked_length(length, "lset", MAX_LENGTH)?;
    let fill = pad.chars().next().unwrap_or(' ');
    let all = chars(text);
    let mut result: String = all.iter().take(width).collect();
    for _ in all.len().min(width)..width {
        result.push(fill);
    }
    Ok(result)
}

/// Right-aligns `text` in a field of `length`, padding on the left with the
/// first character of `pad` (PureBasic `RSet`).
///
/// A `text` longer than `length` keeps its last `length` characters, and an
/// empty `pad` pads with spaces.
pub fn rset(text: &str, length: i64, pad: &str) -> Result<String, StringError> {
    let width = checked_length(length, "rset", MAX_LENGTH)?;
    let fill = pad.chars().next().unwrap_or(' ');
    let all = chars(text);
    let visible = all.len().min(width);
    let mut result = String::new();
    for _ in 0..width - visible {
        result.push(fill);
    }
    result.extend(all[all.len() - visible..].iter());
    Ok(result)
}

#[cfg(test)]
mod tests;
