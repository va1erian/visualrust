//! Proleptic Gregorian calendar conversions.
//!
//! The civil-date algorithms are Howard Hinnant's `days_from_civil` /
//! `civil_from_days` (<https://howardhinnant.github.io/date_algorithms.html>).
//! They are exact across the whole proleptic Gregorian range and avoid pulling
//! a calendar crate in for what is a few dozen lines of arithmetic. Callers
//! keep years within `1..=9999`, so every intermediate stays far from `i64`
//! overflow.

/// Number of whole seconds in a UTC day.
pub(crate) const SECONDS_PER_DAY: i64 = 86_400;

const MILLIS_PER_SECOND: i64 = 1_000;
const SECONDS_PER_HOUR: i64 = 3_600;
const SECONDS_PER_MINUTE: i64 = 60;

/// A broken-down UTC instant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Civil {
    /// Proleptic Gregorian year.
    pub(crate) year: i64,
    /// Month in `1..=12`.
    pub(crate) month: u32,
    /// Day of month in `1..=days_in_month`.
    pub(crate) day: u32,
    /// Hour in `0..=23`.
    pub(crate) hour: u32,
    /// Minute in `0..=59`.
    pub(crate) minute: u32,
    /// Second in `0..=59`.
    pub(crate) second: u32,
    /// Millisecond in `0..=999`.
    pub(crate) millisecond: u32,
}

/// Splits a Unix timestamp in milliseconds into its UTC civil fields.
///
/// Negative instants (before 1970) use Euclidean division, so the result is
/// floored and `-1` ms is the last millisecond of 1969-12-31 rather than
/// rounding toward zero.
pub(crate) fn from_unix_millis(ms: i64) -> Civil {
    let seconds = ms.div_euclid(MILLIS_PER_SECOND);
    let millisecond = ms.rem_euclid(MILLIS_PER_SECOND) as u32;
    let days = seconds.div_euclid(SECONDS_PER_DAY);
    let seconds_of_day = seconds.rem_euclid(SECONDS_PER_DAY);
    let (year, month, day) = civil_from_days(days);
    Civil {
        year,
        month,
        day,
        hour: (seconds_of_day / SECONDS_PER_HOUR) as u32,
        minute: ((seconds_of_day % SECONDS_PER_HOUR) / SECONDS_PER_MINUTE) as u32,
        second: (seconds_of_day % SECONDS_PER_MINUTE) as u32,
        millisecond,
    }
}

/// Combines civil fields into a Unix timestamp in milliseconds.
pub(crate) fn to_unix_millis(civil: &Civil) -> i64 {
    let days = days_from_civil(civil.year, civil.month, civil.day);
    let seconds = days * SECONDS_PER_DAY
        + i64::from(civil.hour) * SECONDS_PER_HOUR
        + i64::from(civil.minute) * SECONDS_PER_MINUTE
        + i64::from(civil.second);
    seconds * MILLIS_PER_SECOND + i64::from(civil.millisecond)
}

/// Number of days in `month` of `year`, using the Gregorian leap rule.
pub(crate) fn days_in_month(year: i64, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap(year) => 29,
        2 => 28,
        // Parsing validates the month before calling this; 0 keeps the function
        // total without panicking.
        _ => 0,
    }
}

fn is_leap(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    let year = year + i64::from(month <= 2);
    (year, month as u32, day as u32)
}

fn days_from_civil(year: i64, month: u32, day: u32) -> i64 {
    let month = i64::from(month);
    let day = i64::from(day);
    let year = year - i64::from(month <= 2);
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let yoe = year - era * 400;
    let doy = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}
