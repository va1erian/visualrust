//! A Dyon script exercises several string commands through the shared runtime,
//! proving `vr_std::register` wires them into `from_source_with`.

use vr_dyon::DyonRuntime;

/// The script concatenates every result so the test can assert on one string.
///
/// The PureBasic commands register under `num_str`, `str_len` and `str_trim`
/// because the bare names would shadow Dyon's built-ins; see #104.
const SCRIPT: &str = r#"
fn run() -> str {
    return left("VisualRust", 6) +
        "|" + right("VisualRust", 4) +
        "|" + mid("VisualRust", 7, 4) +
        "|" + ucase("dyn") +
        "|" + lcase("DYn") +
        "|" + num_str(find_string("VisualRust", "Rust")) +
        "|" + replace_string("a-b-c", "-", "+") +
        "|" + str_trim("  hi  ") +
        "|" + num_str(count_string("banana", "an")) +
        "|" + insert_string("ab", 2, "-") +
        "|" + remove_string("a-b-c", 2, 1) +
        "|" + string_field("a,b,c", 2, ",") +
        "|" + space(3) +
        "|" + num_str(val("42abc")) +
        "|" + num_str(str_len("héllo")) +
        "|" + hex(255, 2) +
        "|" + bin(5, 4) +
        "|" + format(3.5, 2) +
        "|" + lset("ab", 4, "0") +
        "|" + rset("ab", 4, "0")
}
fn main() {}
"#;

const EXPECTED: &str =
    "Visual|Rust|Rust|DYN|dyn|7|a+b+c|hi|2|a-b|ab-c|b|   |42|5|FF|0101|3.50|ab00|00ab";

#[test]
fn dyon_script_uses_the_string_library() {
    let mut runtime =
        DyonRuntime::from_source_with("strings_end_to_end.dyon", SCRIPT, vr_std::register)
            .expect("program compiles with the native signatures");
    let result: String = runtime.call_ret("run", &[]).expect("script runs");
    assert_eq!(result, EXPECTED);
}

#[test]
fn a_bad_position_surfaces_as_a_runtime_error() {
    let source = r#"
fn run() -> str {
    return mid("abc", 9, 1)
}
fn main() {}
"#;
    let mut runtime =
        DyonRuntime::from_source_with("strings_bad_index.dyon", source, vr_std::register)
            .expect("program compiles");
    assert!(runtime.call_ret::<String>("run", &[]).is_err());
}
