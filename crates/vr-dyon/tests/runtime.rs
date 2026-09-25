use vr_dyon::{DyonError, DyonRuntime, PushVariable};

const NATIVE_PROGRAM: &str = r#"
fn main() {
    answer := vr_native_add(2, 3)
    message := vr_native_echo("hello")
}
"#;

#[test]
fn native_function_is_callable_from_dyon() {
    let mut runtime =
        DyonRuntime::from_source("native.dyon", NATIVE_PROGRAM).expect("program loads");
    runtime.run().expect("native call succeeds");
}

#[test]
fn native_function_round_trips_a_value() {
    let mut runtime =
        DyonRuntime::from_source("empty.dyon", "fn main() {}\n").expect("program loads");
    let args = [2.0_f64.push_var(), 3.0_f64.push_var()];
    let sum: f64 = runtime
        .call_ret("vr_native_add", &args)
        .expect("native returns");
    assert_eq!(sum, 5.0);
}

#[test]
fn loads_program_from_file() {
    let path = std::env::temp_dir().join(format!("vr-dyon-{}-native.dyon", std::process::id()));
    std::fs::write(&path, NATIVE_PROGRAM).expect("write fixture");
    let loaded = DyonRuntime::from_file(&path);
    let _ = std::fs::remove_file(&path);
    let mut runtime = loaded.expect("program loads from file");
    runtime.run().expect("native call succeeds");
}

#[test]
fn missing_file_is_a_typed_io_error() {
    let error = DyonRuntime::from_file("vr-dyon-definitely-missing.dyon")
        .err()
        .expect("load must fail");
    assert!(matches!(error, DyonError::Io { .. }), "got {error:?}");
}

#[test]
fn compile_error_reports_source_line() {
    let source = "fn main() {\n    x := )\n}\n";
    let error = DyonRuntime::from_source("compile.dyon", source)
        .err()
        .expect("compile must fail");
    match error {
        DyonError::Compile { position, .. } => {
            assert_eq!(position.expect("position is present").line, 2);
        }
        other => panic!("expected a compile error, got {other:?}"),
    }
}

#[test]
fn runtime_error_reports_source_line() {
    let source = "fn main() {\n    x := 1\n    x = \"two\"\n}\n";
    let mut runtime = DyonRuntime::from_source("runtime.dyon", source).expect("program loads");
    let error = runtime.run().expect_err("run must fail");
    match error {
        DyonError::Runtime { position, .. } => {
            assert_eq!(position.expect("position is present").line, 3);
        }
        other => panic!("expected a runtime error, got {other:?}"),
    }
}
