//! A Dyon script exercises several math commands through the shared runtime,
//! proving `vr_std::register` wires them into `from_source_with`.
//!
//! Every command that shares a Dyon built-in name registers under a `num_`
//! prefix; see #104. Unexported `str` calls use Dyon's own built-in, and
//! `sqrt(2)` goes through `format` so the fractional digits are stable.

use vr_dyon::DyonRuntime;

const SCRIPT: &str = r#"
fn run() -> str {
    return num_str(num_abs(-3.5)) +
        "|" + num_str(sign(-2)) +
        "|" + num_str(sign(0)) +
        "|" + num_str(int(-3.9)) +
        "|" + num_str(num_round(2.5)) +
        "|" + num_str(num_floor(-1.5)) +
        "|" + num_str(num_ceil(1.2)) +
        "|" + num_str(num_pow(2, 10)) +
        "|" + format(num_sqrt(2), 4) +
        "|" + num_str(num_atan2(0, 1)) +
        "|" + num_str(num_exp(0)) +
        "|" + num_str(num_ln(1)) +
        "|" + num_str(num_log10(1000)) +
        "|" + num_str(num_log2(8)) +
        "|" + num_str(num_min(3, 7)) +
        "|" + num_str(num_max(3, 7)) +
        "|" + num_str(num_sin(0)) +
        "|" + num_str(num_cos(0)) +
        "|" + num_str(num_atan(0)) +
        "|" + num_str(num_asin(0)) +
        "|" + num_str(num_acos(1))
}
fn main() {}
"#;

const EXPECTED: &str = "3.5|-1|0|-3|3|-2|2|1024|1.4142|0|1|0|3|3|3|7|0|1|0|0|0";

#[test]
fn dyon_script_uses_the_math_library() {
    let mut runtime =
        DyonRuntime::from_source_with("math_end_to_end.dyon", SCRIPT, vr_std::register)
            .expect("program compiles with the native signatures");
    let result: String = runtime.call_ret("run", &[]).expect("script runs");
    assert_eq!(result, EXPECTED);
}

#[test]
fn a_domain_violation_surfaces_as_a_runtime_error() {
    let source = r#"
fn run() -> f64 {
    return num_sqrt(-1)
}
fn main() {}
"#;
    let mut runtime =
        DyonRuntime::from_source_with("math_bad_domain.dyon", source, vr_std::register)
            .expect("program compiles");
    assert!(runtime.call_ret::<f64>("run", &[]).is_err());
}

#[test]
fn seeded_random_is_reproducible_through_dyon() {
    let source = r#"
fn run() -> str {
    random_seed(123)
    return num_str(num_random()) + "|" + num_str(num_random()) + "|" + num_str(num_random())
}
fn main() {}
"#;
    let mut runtime = DyonRuntime::from_source_with("math_random.dyon", source, vr_std::register)
        .expect("program compiles");
    let first: String = runtime.call_ret("run", &[]).expect("script runs");
    let second: String = runtime.call_ret("run", &[]).expect("script runs again");
    assert_eq!(first, second);
}
