//! PureBasic-style math over Dyon's `f64` numbers.
//!
//! Dyon has exactly one number type, `f64`, so every command here takes and
//! returns `f64`. The integer-valued commands ([`int`], [`round`], [`floor`],
//! [`ceil`]) return a `f64` with no fractional part; Dyon converts that to an
//! index or count on its own.
//!
//! # Rounding
//!
//! The four rounding commands differ only at the boundaries:
//!
//! | command   | rule                            | `2.5` | `-2.5` | `2.4` | `-2.4` |
//! |-----------|---------------------------------|-------|--------|-------|--------|
//! | [`int`]   | truncate toward zero            | `2`   | `-2`   | `2`   | `-2`   |
//! | [`floor`] | round toward negative infinity  | `2`   | `-3`   | `2`   | `-3`   |
//! | [`ceil`]  | round toward positive infinity  | `3`   | `-2`   | `3`   | `-2`   |
//! | [`round`] | half away from zero             | `3`   | `-3`   | `2`   | `-2`   |
//!
//! # Domains
//!
//! [`sqrt`], [`ln`], [`log10`] and [`log2`] reject a value outside their real
//! domain with a typed [`MathError::Domain`], and [`asin`]/[`acos`] reject a
//! value outside `[-1, 1]`. A NaN argument is *not* a domain error: it
//! propagates to a NaN result, matching IEEE 754 and the way `+`, `-`, `*`
//! already behave in a script.
//!
//! # Randomness
//!
//! [`random`] returns a float in `[0, 1)`. The generator is a 64-bit
//! xorshift64* seeded from the IEEE-754 bit pattern of the value passed to
//! [`random_seed`]; it is deliberately small and dependency-free so scripts can
//! reproduce a sequence across runs and machines. It is **not** suitable for
//! cryptographic use.

use std::sync::{Mutex, MutexGuard, OnceLock};

use crate::error::MathError;

/// Seed used until a script calls [`random_seed`].
///
/// Non-zero because xorshift would stay at zero forever; the value is an
/// arbitrary constant so an unseeded run is still deterministic.
pub const DEFAULT_SEED: u64 = 0x9E37_79B9_7F4A_7C15;

/// A small, deterministic xorshift64* pseudo-random generator.
///
/// Exposed as a value so unit tests can check reproducibility without racing
/// on the process-wide generator behind [`random`].
#[derive(Debug, Clone)]
pub struct Prng {
    state: u64,
}

impl Prng {
    /// Creates a generator from `seed`.
    ///
    /// A zero state is replaced with [`DEFAULT_SEED`]: xorshift maps zero to
    /// zero, which would yield a constant stream instead of random values.
    pub fn new(seed: u64) -> Self {
        let state = if seed == 0 { DEFAULT_SEED } else { seed };
        Prng { state }
    }

    /// Creates a generator from the bits of a Dyon number.
    ///
    /// Using the bit pattern instead of `as u64` keeps fractional and negative
    /// seeds distinct rather than saturating them all to `0`.
    pub fn from_f64(seed: f64) -> Self {
        Prng::new(seed.to_bits())
    }

    /// Returns the next value in `[0, 1)`.
    ///
    /// Only the top 53 bits are used, which is the full mantissa of an `f64`,
    /// so the result has unit spacing and includes `0.0` but never `1.0`.
    pub fn next_f64(&mut self) -> f64 {
        let mut x = self.state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.state = x;
        let scrambled = x.wrapping_mul(0x2545_F491_4F6C_DD1D);
        (scrambled >> 11) as f64 / (1u64 << 53) as f64
    }
}

/// Process-wide generator behind [`random`] and [`random_seed`].
static STATE: OnceLock<Mutex<Prng>> = OnceLock::new();

/// Locks the process-wide generator, recovering from a poisoned mutex.
///
/// A panic while holding the lock cannot leave the generator in an invalid
/// state (it is a single integer), so recovering is safer than panicking in a
/// command that promises not to.
fn rng() -> MutexGuard<'static, Prng> {
    STATE
        .get_or_init(|| Mutex::new(Prng::new(DEFAULT_SEED)))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Reseeds the process-wide generator (PureBasic `RandomSeed`).
pub fn random_seed(seed: f64) {
    *rng() = Prng::from_f64(seed);
}

/// Returns the next float in `[0, 1)` (PureBasic `Random`).
pub fn random() -> f64 {
    rng().next_f64()
}

/// Builds a domain error for a wrapper.
fn domain(function: &'static str, value: f64) -> MathError {
    MathError::Domain { function, value }
}

/// Absolute value (PureBasic `Abs`).
pub fn abs(value: f64) -> f64 {
    value.abs()
}

/// Sign of `value`: `-1`, `0` or `1` (PureBasic `Sign`).
///
/// Negative zero is treated as zero; a NaN returns NaN, mirroring the
/// propagating-NaN rule used by the domain-checked functions.
pub fn sign(value: f64) -> f64 {
    if value.is_nan() {
        f64::NAN
    } else if value > 0.0 {
        1.0
    } else if value < 0.0 {
        -1.0
    } else {
        0.0
    }
}

/// Truncates the fractional part, toward zero (PureBasic `Int`).
pub fn int(value: f64) -> f64 {
    value.trunc()
}

/// Rounds to the nearest integer, half away from zero (PureBasic `Round`).
pub fn round(value: f64) -> f64 {
    value.round()
}

/// Rounds toward negative infinity (PureBasic `Floor`).
pub fn floor(value: f64) -> f64 {
    value.floor()
}

/// Rounds toward positive infinity (PureBasic `Ceil`).
pub fn ceil(value: f64) -> f64 {
    value.ceil()
}

/// `base` raised to `exponent` (PureBasic `Pow`).
///
/// Overflow yields an infinity and an undefined real power yields NaN, matching
/// IEEE 754 rather than raising a domain error.
pub fn pow(base: f64, exponent: f64) -> f64 {
    base.powf(exponent)
}

/// Square root, rejecting a negative argument (PureBasic `Sqrt`).
pub fn sqrt(value: f64) -> Result<f64, MathError> {
    if value < 0.0 {
        Err(domain("sqrt", value))
    } else {
        Ok(value.sqrt())
    }
}

/// Sine of `value` radians (PureBasic `Sin`).
pub fn sin(value: f64) -> f64 {
    value.sin()
}

/// Cosine of `value` radians (PureBasic `Cos`).
pub fn cos(value: f64) -> f64 {
    value.cos()
}

/// Tangent of `value` radians (PureBasic `Tan`).
pub fn tan(value: f64) -> f64 {
    value.tan()
}

/// Arc sine in radians, rejecting a value outside `[-1, 1]` (PureBasic `ASin`).
pub fn asin(value: f64) -> Result<f64, MathError> {
    if !value.is_nan() && !(-1.0..=1.0).contains(&value) {
        Err(domain("asin", value))
    } else {
        Ok(value.asin())
    }
}

/// Arc cosine in radians, rejecting a value outside `[-1, 1]`
/// (PureBasic `ACos`).
pub fn acos(value: f64) -> Result<f64, MathError> {
    if !value.is_nan() && !(-1.0..=1.0).contains(&value) {
        Err(domain("acos", value))
    } else {
        Ok(value.acos())
    }
}

/// Arc tangent in radians (PureBasic `ATan`).
pub fn atan(value: f64) -> f64 {
    value.atan()
}

/// Angle of the point `(x, y)` in radians, using the signs of both arguments
/// (PureBasic `ATan2`).
///
/// Follows the usual `atan2(y, x)` order, so `atan2(0, 1)` is `0`.
pub fn atan2(y: f64, x: f64) -> f64 {
    y.atan2(x)
}

/// Natural logarithm, rejecting a non-positive argument (PureBasic `Log`).
pub fn ln(value: f64) -> Result<f64, MathError> {
    if value <= 0.0 {
        Err(domain("ln", value))
    } else {
        Ok(value.ln())
    }
}

/// Base-10 logarithm, rejecting a non-positive argument.
pub fn log10(value: f64) -> Result<f64, MathError> {
    if value <= 0.0 {
        Err(domain("log10", value))
    } else {
        Ok(value.log10())
    }
}

/// Base-2 logarithm, rejecting a non-positive argument.
pub fn log2(value: f64) -> Result<f64, MathError> {
    if value <= 0.0 {
        Err(domain("log2", value))
    } else {
        Ok(value.log2())
    }
}

/// `e` raised to `value` (PureBasic `Exp`).
pub fn exp(value: f64) -> f64 {
    value.exp()
}

/// Smaller of two values (PureBasic `Min`).
///
/// One NaN operand is ignored in favour of the other, matching `f64::min`.
pub fn min(a: f64, b: f64) -> f64 {
    a.min(b)
}

/// Larger of two values (PureBasic `Max`).
///
/// One NaN operand is ignored in favour of the other, matching `f64::max`.
pub fn max(a: f64, b: f64) -> f64 {
    a.max(b)
}

#[cfg(test)]
mod tests;
