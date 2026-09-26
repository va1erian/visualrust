//! PureBasic-style date and time commands over a pluggable clock.
//!
//! # UTC by default
//!
//! Every command here works in **UTC**. `std` cannot read the machine's local
//! time zone without platform-specific code, and pulling in a time-zone
//! database would dwarf the rest of the standard library, so the library makes
//! one documented choice instead of a half-supported local mode: [`date`] and
//! [`time`] report UTC, and [`format_date`]/[`parse_date`] interpret their
//! instants as UTC. A script that needs a local wall clock applies the offset
//! itself. The names are deliberately unprefixed so the default is the only
//! behaviour, not one of two subtly different ones.
//!
//! # Clocks
//!
//! Wall-clock reads and monotonic timings come from a [`Clock`]. Production
//! code uses [`SystemClock`]; tests substitute a frozen or manual clock so that
//! [`date`], [`time`] and [`elapsed_milliseconds`] become deterministic. The
//! same seam is why [`format_date`] and [`parse_date`] take an explicit instant
//! rather than reading the clock: their assertions never depend on when the
//! suite runs.
//!
//! [`elapsed_milliseconds`] is a **monotonic** reading measured from the start
//! of the process. It never goes backwards and ignores wall-clock changes, so
//! it is the right tool for measuring durations — but it is process-relative,
//! not an absolute time.

use std::sync::OnceLock;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::error::DateTimeError;

mod calendar;
mod format;

pub use format::{format_date, parse_date};

use calendar::SECONDS_PER_DAY;

/// Earliest instant [`format_date`] accepts: `0001-01-01T00:00:00Z`.
pub const MIN_TIMESTAMP: i64 = -62_135_596_800;

/// Latest instant [`format_date`] accepts: `9999-12-31T23:59:59Z`.
pub const MAX_TIMESTAMP: i64 = 253_402_300_799;

/// Source of wall-clock and monotonic time.
///
/// The methods take `&self` so a test clock can hand out fixed values without
/// mutable state; anything that really advances must use interior mutability.
pub trait Clock {
    /// Milliseconds since the Unix epoch, UTC. May be negative before 1970.
    fn wall_millis(&self) -> i64;

    /// Milliseconds since the clock's process start. Monotonic.
    fn elapsed_millis(&self) -> u64;

    /// Blocks the current thread for at least `millis` milliseconds.
    fn sleep(&self, millis: u64);
}

/// The real clock: `SystemTime` for wall time, `Instant` for monotonic time.
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemClock;

/// Captured on the first monotonic read so elapsed time is process-relative.
static PROCESS_START: OnceLock<Instant> = OnceLock::new();

impl Clock for SystemClock {
    fn wall_millis(&self) -> i64 {
        match SystemTime::now().duration_since(UNIX_EPOCH) {
            Ok(since_epoch) => i64::try_from(since_epoch.as_millis()).unwrap_or(i64::MAX),
            // A clock set before 1970 yields a negative offset, not a failure.
            Err(before_epoch) => {
                -i64::try_from(before_epoch.duration().as_millis()).unwrap_or(i64::MAX)
            }
        }
    }

    fn elapsed_millis(&self) -> u64 {
        let start = *PROCESS_START.get_or_init(Instant::now);
        u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX)
    }

    fn sleep(&self, millis: u64) {
        std::thread::sleep(Duration::from_millis(millis));
    }
}

/// Seconds since the Unix epoch, UTC (PureBasic `Date`).
///
/// Truncated toward negative infinity by [`i64::div_euclid`], so the instant
/// just before the epoch reports `-1` rather than `0`.
pub fn date(clock: &dyn Clock) -> i64 {
    clock.wall_millis().div_euclid(1_000)
}

/// Seconds since UTC midnight for the current instant (PureBasic `Time`).
///
/// Always in `0..86_400`, independent of the day. UTC, like every command here.
pub fn time(clock: &dyn Clock) -> i64 {
    date(clock).rem_euclid(SECONDS_PER_DAY)
}

/// Monotonic milliseconds since the process started.
///
/// Process-relative and unaffected by wall-clock adjustments; use it to
/// measure durations, not to tell the time.
pub fn elapsed_milliseconds(clock: &dyn Clock) -> f64 {
    clock.elapsed_millis() as f64
}

/// Blocks for at least `millis` milliseconds (PureBasic `Delay`).
///
/// The OS timer granularity may round the sleep up; it never returns early by
/// design of [`std::thread::sleep`].
pub fn delay(clock: &dyn Clock, millis: u64) {
    clock.sleep(millis);
}

/// Converts a Dyon number to a whole-second UTC timestamp.
///
/// Dyon has only `f64`, so a timestamp arrives as a float; a fractional or
/// non-finite value is not an instant. Range checking is left to
/// [`format_date`], which reports [`DateTimeError::TimestampOutOfRange`].
pub(crate) fn to_instant(value: f64, function: &'static str) -> Result<i64, DateTimeError> {
    if value.is_finite() && value.fract() == 0.0 {
        Ok(value as i64)
    } else {
        Err(DateTimeError::InvalidArgument {
            function,
            value,
            expected: "a whole-second UTC timestamp",
        })
    }
}

/// Converts a Dyon number to a non-negative whole-millisecond count.
pub(crate) fn to_millis(value: f64, function: &'static str) -> Result<u64, DateTimeError> {
    if value.is_finite() && value.fract() == 0.0 && value >= 0.0 && value <= u64::MAX as f64 {
        Ok(value as u64)
    } else {
        Err(DateTimeError::InvalidArgument {
            function,
            value,
            expected: "a non-negative whole-millisecond count",
        })
    }
}

#[cfg(test)]
mod tests;
