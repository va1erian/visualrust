//! Unit tests for the pure math API.
//!
//! These call the Rust functions directly, so a fractional comparison failure
//! points at the math logic rather than at Dyon's marshalling. The helper takes
//! an absolute tolerance because the trig results are irrational.

use super::*;

/// Asserts two floats are equal within `tolerance`.
fn assert_close(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "expected {expected}, got {actual}"
    );
}

/// Unwraps the error of a failing call for comparison.
fn failure<T>(result: Result<T, MathError>) -> MathError {
    match result {
        Ok(_) => panic!("expected a MathError"),
        Err(error) => error,
    }
}

#[test]
fn abs_and_sign_handle_negatives_and_zero() {
    assert_eq!(abs(-3.5), 3.5);
    assert_eq!(abs(3.5), 3.5);
    assert_eq!(sign(-2.0), -1.0);
    assert_eq!(sign(2.0), 1.0);
    assert_eq!(sign(0.0), 0.0);
    assert_eq!(sign(-0.0), 0.0);
    assert!(sign(f64::NAN).is_nan());
}

#[test]
fn integer_commands_apply_the_documented_rounding() {
    assert_eq!(int(2.5), 2.0);
    assert_eq!(int(-2.5), -2.0);
    assert_eq!(int(2.9), 2.0);
    assert_eq!(int(-2.9), -2.0);

    assert_eq!(floor(2.5), 2.0);
    assert_eq!(floor(-2.5), -3.0);
    assert_eq!(floor(2.0), 2.0);

    assert_eq!(ceil(2.5), 3.0);
    assert_eq!(ceil(-2.5), -2.0);
    assert_eq!(ceil(2.0), 2.0);

    assert_eq!(round(2.4), 2.0);
    assert_eq!(round(2.5), 3.0);
    assert_eq!(round(-2.4), -2.0);
    assert_eq!(round(-2.5), -3.0);
}

#[test]
fn pow_and_sqrt_return_known_values() {
    assert_close(pow(2.0, 10.0), 1024.0, 0.0);
    assert_close(pow(9.0, 0.5), 3.0, 1e-12);
    assert_close(sqrt(16.0).expect("non-negative"), 4.0, 0.0);
    assert_close(
        sqrt(2.0).expect("non-negative"),
        std::f64::consts::SQRT_2,
        1e-12,
    );
}

#[test]
fn trig_returns_known_values() {
    assert_close(sin(0.0), 0.0, 0.0);
    assert_close(cos(0.0), 1.0, 0.0);
    assert_close(tan(0.0), 0.0, 0.0);
    assert_close(atan(0.0), 0.0, 0.0);
    let third_pi = std::f64::consts::PI / 3.0;
    assert_close(sin(third_pi), 0.866_025_403_78, 1e-9);
    assert_close(cos(third_pi), 0.5, 1e-9);
    assert_close(atan2(1.0, 1.0), std::f64::consts::FRAC_PI_4, 1e-12);
    assert_close(atan2(0.0, 1.0), 0.0, 0.0);
}

#[test]
fn logs_and_exp_return_known_values() {
    assert_close(ln(1.0).expect("positive"), 0.0, 0.0);
    assert_close(ln(std::f64::consts::E).expect("positive"), 1.0, 1e-12);
    assert_close(log10(1000.0).expect("positive"), 3.0, 1e-12);
    assert_close(log2(8.0).expect("positive"), 3.0, 1e-12);
    assert_close(exp(0.0), 1.0, 0.0);
    assert_close(exp(1.0), std::f64::consts::E, 1e-12);
}

#[test]
fn min_and_max_choose_the_expected_operand() {
    assert_eq!(min(3.0, 7.0), 3.0);
    assert_eq!(max(3.0, 7.0), 7.0);
    assert_eq!(min(-1.0, 1.0), -1.0);
    // A NaN operand is ignored, matching `f64::min`/`f64::max`.
    assert_eq!(min(f64::NAN, 4.0), 4.0);
    assert_eq!(max(4.0, f64::NAN), 4.0);
}

#[test]
fn sqrt_rejects_negative_arguments() {
    assert_eq!(
        failure(sqrt(-1.0)),
        MathError::Domain {
            function: "sqrt",
            value: -1.0
        }
    );
}

#[test]
fn logarithms_reject_non_positive_arguments() {
    assert_eq!(
        failure(ln(0.0)),
        MathError::Domain {
            function: "ln",
            value: 0.0
        }
    );
    assert_eq!(
        failure(ln(-2.0)),
        MathError::Domain {
            function: "ln",
            value: -2.0
        }
    );
    assert_eq!(
        failure(log10(-1.0)),
        MathError::Domain {
            function: "log10",
            value: -1.0
        }
    );
    assert_eq!(
        failure(log2(0.0)),
        MathError::Domain {
            function: "log2",
            value: 0.0
        }
    );
}

#[test]
fn asin_and_acos_reject_values_outside_the_unit_interval() {
    assert_eq!(
        failure(asin(2.0)),
        MathError::Domain {
            function: "asin",
            value: 2.0
        }
    );
    assert_eq!(
        failure(acos(-1.5)),
        MathError::Domain {
            function: "acos",
            value: -1.5
        }
    );
    assert_close(
        asin(1.0).expect("inside"),
        std::f64::consts::FRAC_PI_2,
        1e-12,
    );
    assert_close(acos(1.0).expect("inside"), 0.0, 0.0);
}

#[test]
fn nan_propagates_instead_of_being_a_domain_error() {
    assert!(sqrt(f64::NAN).expect("NaN passes").is_nan());
    assert!(ln(f64::NAN).expect("NaN passes").is_nan());
    assert!(asin(f64::NAN).expect("NaN passes").is_nan());
}

#[test]
fn prng_is_reproducible_for_a_given_seed() {
    let mut first = Prng::new(42);
    let mut second = Prng::new(42);
    let left: Vec<f64> = (0..8).map(|_| first.next_f64()).collect();
    let right: Vec<f64> = (0..8).map(|_| second.next_f64()).collect();
    assert_eq!(left, right);
    // A different seed must not replay the same sequence.
    let mut other = Prng::new(43);
    assert_ne!(left, (0..8).map(|_| other.next_f64()).collect::<Vec<_>>());
}

#[test]
fn prng_values_stay_within_the_unit_interval() {
    let mut prng = Prng::new(7);
    for _ in 0..1_000 {
        let value = prng.next_f64();
        assert!((0.0..1.0).contains(&value), "out of range: {value}");
    }
}

#[test]
fn prng_seed_zero_is_replaced_so_the_stream_moves() {
    let mut prng = Prng::new(0);
    let first = prng.next_f64();
    let second = prng.next_f64();
    assert_ne!(first, second);
}

#[test]
fn floating_point_seeds_are_not_collapsed_to_integers() {
    // `1.5` and `1` differ only above the integer part; the bit-pattern seed
    // must keep them distinct.
    let mut half = Prng::from_f64(1.5);
    let mut one = Prng::from_f64(1.0);
    assert_ne!(half.next_f64(), one.next_f64());
}

#[test]
fn global_random_stays_within_the_unit_interval() {
    for _ in 0..100 {
        let value = random();
        assert!((0.0..1.0).contains(&value), "out of range: {value}");
    }
}
