//! A Dyon script exercises the date/time commands through the shared runtime,
//! proving `vr_std::register` wires them into `from_source_with`.
//!
//! Every assertion uses an explicit instant or a bounded monotonic reading, so
//! nothing here depends on the wall clock at run time.

use vr_dyon::DyonRuntime;

#[test]
fn dyon_script_formats_and_parses_fixed_instants() {
    let source = r#"
fn run() -> str {
    return format_date(0, "%Y-%m-%d %H:%M:%S") +
        "|" + format_date(1700000000, "%d/%m/%Y %H:%M")
}
fn main() {}
"#;
    let mut runtime =
        DyonRuntime::from_source_with("datetime_format.dyon", source, vr_std::register)
            .expect("program compiles with the native signatures");
    let result: String = runtime.call_ret("run", &[]).expect("script runs");
    assert_eq!(result, "1970-01-01 00:00:00|14/11/2023 22:13");
}

#[test]
fn dyon_parse_date_returns_the_epoch_second() {
    let source = r#"
fn run() -> f64 {
    return parse_date("2023-11-14 22:13:20", "%Y-%m-%d %H:%M:%S")
}
fn main() {}
"#;
    let mut runtime =
        DyonRuntime::from_source_with("datetime_parse.dyon", source, vr_std::register)
            .expect("program compiles");
    let result: f64 = runtime.call_ret("run", &[]).expect("script runs");
    assert_eq!(result, 1_700_000_000.0);
}

#[test]
fn dyon_delay_and_elapsed_milliseconds_are_usable() {
    let source = r#"
fn run() -> f64 {
    delay(1)
    return elapsed_milliseconds()
}
fn main() {}
"#;
    let mut runtime =
        DyonRuntime::from_source_with("datetime_clock.dyon", source, vr_std::register)
            .expect("program compiles");
    let result: f64 = runtime.call_ret("run", &[]).expect("script runs");
    assert!(result >= 0.0, "elapsed was {result}");
}

#[test]
fn a_bad_format_surfaces_as_a_runtime_error() {
    let source = r#"
fn run() -> str {
    return format_date(0, "%q")
}
fn main() {}
"#;
    let mut runtime = DyonRuntime::from_source_with("datetime_bad.dyon", source, vr_std::register)
        .expect("program compiles");
    assert!(runtime.call_ret::<String>("run", &[]).is_err());
}
