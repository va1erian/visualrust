//! Unit tests for the pure date/time API.
//!
//! These drive the Rust functions directly and substitute a [`FrozenClock`] for
//! the wall clock, so every assertion is deterministic: no test reads the real
//! system time. Only the monotonic and sleep tests touch [`SystemClock`], and
//! they assert bounds rather than exact values.

use std::time::{Duration, Instant};

use super::*;

/// A clock frozen at one wall instant and one monotonic reading.
struct FrozenClock {
    wall: i64,
    elapsed: u64,
}

impl Clock for FrozenClock {
    fn wall_millis(&self) -> i64 {
        self.wall
    }

    fn elapsed_millis(&self) -> u64 {
        self.elapsed
    }

    fn sleep(&self, _millis: u64) {}
}

/// A reference instant with a non-zero result in every field:
/// `2023-11-14T22:13:20Z`.
const REFERENCE_SECONDS: i64 = 1_700_000_000;
const REFERENCE_MILLIS: i64 = REFERENCE_SECONDS * 1_000;

#[test]
fn format_renders_known_utc_instants() {
    assert_eq!(
        format_date(0, "%Y-%m-%d %H:%M:%S").expect("in range"),
        "1970-01-01 00:00:00"
    );
    assert_eq!(
        format_date(REFERENCE_SECONDS, "%Y-%m-%d %H:%M:%S").expect("in range"),
        "2023-11-14 22:13:20"
    );
    assert_eq!(
        format_date(REFERENCE_SECONDS, "%d/%m/%Y %H:%M").expect("in range"),
        "14/11/2023 22:13"
    );
    // A literal percent is the only escape in the subset.
    assert_eq!(
        format_date(REFERENCE_SECONDS, "100%% at %H:%M").expect("in range"),
        "100% at 22:13"
    );
}

#[test]
fn format_and_parse_round_trip_for_several_formats() {
    // Formats that capture the whole instant round-trip the reference value;
    // a date-only format lands on midnight, so it is checked with a midnight
    // instant to keep the assertion honest.
    let full = [
        "%Y-%m-%d %H:%M:%S",
        "%Y-%m-%dT%H:%M:%S",
        "%d.%m.%Y %H:%M:%S",
        "on %Y-%m-%d at %H:%M:%S",
    ];
    for format in full {
        let text = format_date(REFERENCE_SECONDS, format).expect("in range");
        let parsed = parse_date(&text, format).expect("round-trips");
        assert_eq!(parsed, REFERENCE_SECONDS, "format {format} produced {text}");
    }

    let midnight = REFERENCE_SECONDS - 80_000;
    let text = format_date(midnight, "%Y/%m/%d").expect("in range");
    assert_eq!(text, "2023/11/14");
    assert_eq!(
        parse_date(&text, "%Y/%m/%d").expect("round-trips"),
        midnight
    );
}

#[test]
fn from_unix_millis_handles_negative_instants() {
    // One millisecond before the epoch is the last millisecond of 1969.
    let civil = calendar::from_unix_millis(-1);
    assert_eq!(civil.year, 1969);
    assert_eq!(civil.month, 12);
    assert_eq!(civil.day, 31);
    assert_eq!(civil.hour, 23);
    assert_eq!(civil.minute, 59);
    assert_eq!(civil.second, 59);
    assert_eq!(civil.millisecond, 999);
    assert_eq!(calendar::to_unix_millis(&civil), -1);
}

#[test]
fn leap_days_are_accepted_only_in_leap_years() {
    // 2000-02-29T00:00:00Z.
    assert_eq!(
        parse_date("2000-02-29", "%Y-%m-%d").expect("2000 is a leap year"),
        951_782_400
    );
    assert!(matches!(
        parse_date("2023-02-29", "%Y-%m-%d"),
        Err(DateTimeError::FieldOutOfRange {
            field: "day",
            value: 29,
            ..
        })
    ));
}

#[test]
fn frozen_clock_pins_date_and_time() {
    let clock = FrozenClock {
        wall: REFERENCE_MILLIS,
        elapsed: 0,
    };
    assert_eq!(date(&clock), REFERENCE_SECONDS);
    // 22:13:20 -> 22 * 3600 + 13 * 60 + 20.
    assert_eq!(time(&clock), 80_000);

    // The frozen clock also makes a format assertion independent of the suite's
    // run time.
    let rendered = format_date(date(&clock), "%Y-%m-%d %H:%M:%S").expect("in range");
    assert_eq!(rendered, "2023-11-14 22:13:20");
}

#[test]
fn time_wraps_at_utc_midnight_for_negative_instants() {
    let clock = FrozenClock {
        wall: -1,
        elapsed: 0,
    };
    assert_eq!(date(&clock), -1);
    assert_eq!(time(&clock), 86_399);
}

#[test]
fn parse_rejects_a_text_that_does_not_match_the_format() {
    assert!(matches!(
        parse_date("2023-11-14 trailing", "%Y-%m-%d"),
        Err(DateTimeError::ParseMismatch { position: 10, .. })
    ));
    assert!(matches!(
        parse_date("2023-13-14", "%Y-%m-%d"),
        Err(DateTimeError::FieldOutOfRange {
            field: "month",
            value: 13,
            ..
        })
    ));
    assert!(matches!(
        parse_date("2023-11", "%Y-%m-%d"),
        Err(DateTimeError::ParseMismatch { .. })
    ));
}

#[test]
fn parse_requires_the_date_fields() {
    assert!(matches!(
        parse_date("22:13:20", "%H:%M:%S"),
        Err(DateTimeError::MissingField { field: "year", .. })
    ));
}

#[test]
fn unknown_or_dangling_specifiers_are_rejected() {
    assert!(matches!(
        format_date(0, "%q"),
        Err(DateTimeError::UnknownSpecifier { specifier: 'q', .. })
    ));
    assert!(matches!(
        format_date(0, "ends with %"),
        Err(DateTimeError::DanglingPercent { .. })
    ));
    assert!(matches!(
        parse_date("0", "%q"),
        Err(DateTimeError::UnknownSpecifier { specifier: 'q', .. })
    ));
}

#[test]
fn format_rejects_instants_outside_the_calendar_range() {
    assert!(matches!(
        format_date(MAX_TIMESTAMP + 1, "%Y"),
        Err(DateTimeError::TimestampOutOfRange { .. })
    ));
    assert!(matches!(
        format_date(MIN_TIMESTAMP - 1, "%Y"),
        Err(DateTimeError::TimestampOutOfRange { .. })
    ));
}

#[test]
fn numeric_arguments_must_be_whole_counts() {
    assert!(matches!(
        to_instant(1.5, "format_date"),
        Err(DateTimeError::InvalidArgument { .. })
    ));
    assert!(matches!(
        to_millis(-1.0, "delay"),
        Err(DateTimeError::InvalidArgument { .. })
    ));
    assert_eq!(to_instant(-3.0, "format_date").expect("whole"), -3);
    assert_eq!(to_millis(7.0, "delay").expect("whole"), 7);
}

#[test]
fn elapsed_milliseconds_never_goes_backwards() {
    let clock = SystemClock;
    let first = elapsed_milliseconds(&clock);
    delay(&clock, 2);
    let second = elapsed_milliseconds(&clock);
    assert!(
        second >= first,
        "elapsed went backwards: {first} -> {second}"
    );
}

#[test]
fn delay_blocks_for_at_least_the_requested_milliseconds() {
    let clock = SystemClock;
    let start = Instant::now();
    delay(&clock, 5);
    assert!(
        start.elapsed() >= Duration::from_millis(5),
        "delay returned after {:?}",
        start.elapsed()
    );
}
