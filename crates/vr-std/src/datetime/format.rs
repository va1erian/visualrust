//! Formatting and parsing of UTC instants with a small `strftime`-like subset.
//!
//! The supported specifiers are `%Y %m %d %H %M %S %%`. Any other specifier is
//! a typed error rather than being copied through, so a typo fails loudly
//! instead of silently emitting `%q`.
//!
//! Parsing is strict: the text must match the format exactly, fields have fixed
//! widths (`%Y` four digits, the rest two), and every field is range-checked.
//! Date fields (`%Y %m %d`) are required; a format that omits a time field
//! leaves that field at zero, so `"%Y-%m-%d"` parses to midnight. Both
//! functions operate on the explicit instant they are given and never read the
//! current clock. Instants are whole seconds: PureBasic's date value has
//! second resolution and millisecond formatting would only ever print `000`.

use crate::error::DateTimeError;

use super::calendar::{self, Civil};
use super::{MAX_TIMESTAMP, MIN_TIMESTAMP};

/// Renders `timestamp_seconds` (UTC) according to `format`.
///
/// Rejects an instant outside `MIN_TIMESTAMP..=MAX_TIMESTAMP` and an unknown or
/// dangling `%` specifier.
pub fn format_date(timestamp_seconds: i64, format: &str) -> Result<String, DateTimeError> {
    if !(MIN_TIMESTAMP..=MAX_TIMESTAMP).contains(&timestamp_seconds) {
        return Err(DateTimeError::TimestampOutOfRange {
            function: "format_date",
            value: timestamp_seconds,
        });
    }
    let civil = calendar::from_unix_millis(timestamp_seconds * 1_000);
    let mut rendered = String::with_capacity(format.len() + 24);
    let mut specifiers = format.chars();
    while let Some(character) = specifiers.next() {
        if character != '%' {
            rendered.push(character);
            continue;
        }
        let specifier = specifiers.next().ok_or(DateTimeError::DanglingPercent {
            function: "format_date",
        })?;
        match specifier {
            'Y' => rendered.push_str(&format!("{:04}", civil.year)),
            'm' => rendered.push_str(&format!("{:02}", civil.month)),
            'd' => rendered.push_str(&format!("{:02}", civil.day)),
            'H' => rendered.push_str(&format!("{:02}", civil.hour)),
            'M' => rendered.push_str(&format!("{:02}", civil.minute)),
            'S' => rendered.push_str(&format!("{:02}", civil.second)),
            '%' => rendered.push('%'),
            other => {
                return Err(DateTimeError::UnknownSpecifier {
                    function: "format_date",
                    specifier: other,
                });
            }
        }
    }
    Ok(rendered)
}

/// Parses `text` according to `format` and returns the UTC Unix timestamp in
/// seconds.
///
/// The text must match `format` in full; trailing characters are a mismatch.
pub fn parse_date(text: &str, format: &str) -> Result<i64, DateTimeError> {
    let text: Vec<char> = text.chars().collect();
    let pattern: Vec<char> = format.chars().collect();
    let mut fields = Fields::default();
    let mut text_at = 0usize;
    let mut pattern_at = 0usize;

    while pattern_at < pattern.len() {
        let character = pattern[pattern_at];
        pattern_at += 1;
        if character != '%' {
            match text.get(text_at) {
                Some(found) if *found == character => text_at += 1,
                _ => return Err(mismatch(text_at)),
            }
            continue;
        }

        let specifier = pattern
            .get(pattern_at)
            .copied()
            .ok_or(DateTimeError::DanglingPercent {
                function: "parse_date",
            })?;
        pattern_at += 1;
        match specifier {
            'Y' => fields.year = Some(read_digits(&text, &mut text_at, 4)?),
            'm' => fields.month = Some(read_digits(&text, &mut text_at, 2)?),
            'd' => fields.day = Some(read_digits(&text, &mut text_at, 2)?),
            'H' => fields.hour = Some(read_digits(&text, &mut text_at, 2)?),
            'M' => fields.minute = Some(read_digits(&text, &mut text_at, 2)?),
            'S' => fields.second = Some(read_digits(&text, &mut text_at, 2)?),
            '%' => match text.get(text_at) {
                Some('%') => text_at += 1,
                _ => return Err(mismatch(text_at)),
            },
            other => {
                return Err(DateTimeError::UnknownSpecifier {
                    function: "parse_date",
                    specifier: other,
                });
            }
        }
    }

    if text_at != text.len() {
        return Err(mismatch(text_at));
    }
    let civil = fields.into_civil()?;
    Ok(calendar::to_unix_millis(&civil) / 1_000)
}

/// Reads exactly `width` decimal digits, advancing `position`.
fn read_digits(text: &[char], position: &mut usize, width: usize) -> Result<i64, DateTimeError> {
    let mut value = 0i64;
    for _ in 0..width {
        let digit = text
            .get(*position)
            .and_then(|character| character.to_digit(10));
        let Some(digit) = digit else {
            return Err(mismatch(*position));
        };
        value = value * 10 + i64::from(digit);
        *position += 1;
    }
    Ok(value)
}

/// Builds a parse mismatch at character `position`.
fn mismatch(position: usize) -> DateTimeError {
    DateTimeError::ParseMismatch {
        function: "parse_date",
        position,
    }
}

/// The fields a format may name, before validation and defaulting.
#[derive(Default)]
struct Fields {
    year: Option<i64>,
    month: Option<i64>,
    day: Option<i64>,
    hour: Option<i64>,
    minute: Option<i64>,
    second: Option<i64>,
}

impl Fields {
    /// Validates the parsed fields and defaults the optional ones.
    ///
    /// Year, month and day are required because a date without them would be
    /// an arbitrary 1970-01-01; time fields default to midnight.
    fn into_civil(self) -> Result<Civil, DateTimeError> {
        let year = self.year.ok_or_else(|| missing("year"))?;
        let month = self.month.ok_or_else(|| missing("month"))?;
        let day = self.day.ok_or_else(|| missing("day"))?;

        if !(1..=9_999).contains(&year) {
            return Err(out_of_range("year", year));
        }
        if !(1..=12).contains(&month) {
            return Err(out_of_range("month", month));
        }
        let last_day = i64::from(calendar::days_in_month(year, month as u32));
        if !(1..=last_day).contains(&day) {
            return Err(out_of_range("day", day));
        }

        let hour = self.hour.unwrap_or(0);
        let minute = self.minute.unwrap_or(0);
        let second = self.second.unwrap_or(0);
        if !(0..=23).contains(&hour) {
            return Err(out_of_range("hour", hour));
        }
        if !(0..=59).contains(&minute) {
            return Err(out_of_range("minute", minute));
        }
        if !(0..=59).contains(&second) {
            return Err(out_of_range("second", second));
        }

        Ok(Civil {
            year,
            month: month as u32,
            day: day as u32,
            hour: hour as u32,
            minute: minute as u32,
            second: second as u32,
            millisecond: 0,
        })
    }
}

/// Builds a missing-field error for the parser.
fn missing(field: &'static str) -> DateTimeError {
    DateTimeError::MissingField {
        function: "parse_date",
        field,
    }
}

/// Builds an out-of-range error for the parser.
fn out_of_range(field: &'static str, value: i64) -> DateTimeError {
    DateTimeError::FieldOutOfRange {
        function: "parse_date",
        field,
        value,
    }
}
