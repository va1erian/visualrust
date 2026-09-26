//! A Dyon script exercises several math commands through the shared runtime,
//! proving `vr_std::register` wires them into `from_source_with`.
//!
//! Only commands with an exactly representable result are concatenated with
//! `str`; `sqrt(2)` goes through `format` so the fractional digits are stable.

use vr_dyon::DyonRuntime;

const SCRIPT: &str = r#"
fn run() -> str {
    return str(abs(-3.5)) +
        "|" + str(sign(-2)) +
        "|" + str(sign(0)) +
        "|" + str(int(-3.9)) +
        "|" + str(round(2.5)) +
        "|" + str(floor(-1.5)) +
        "|" + str(ceil(1.2)) +
        "|" + str(pow(2, 10)) +
        "|" + format(sqrt(2), 4) +
        "|" + str(atan2(0, 1)) +
        "|" + str(exp(0)) +
        "|" + str(ln(1)) +
        "|" + str(log10(1000)) +
        "|" + str(log2(8)) +
        "|" + str(min(3, 7)) +
        "|" + str(max(3, 7)) +
        "|" + str(sin(0)) +
        "|" + str(cos(0)) +
        "|" + str(atan(0)) +
        "|" + str(asin(0)) +
        "|" + str(acos(1))
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
    return sqrt(-1)
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
    return str(random()) + "|" + str(random()) + "|" + str(random())
}
fn main() {}
"#;
    let mut runtime = DyonRuntime::from_source_with("math_random.dyon", source, vr_std::register)
        .expect("program compiles");
    let first: String = runtime.call_ret("run", &[]).expect("script runs");
    let second: String = runtime.call_ret("run", &[]).expect("script runs again");
    assert_eq!(first, second);
}
