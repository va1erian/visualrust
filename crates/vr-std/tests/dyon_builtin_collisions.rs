//! Regression for #104: `vr_std::register` must not shadow Dyon's built-ins.
//!
//! Dyon resolves a call by name alone (`Module::find_function` scans the
//! external prelude in reverse and returns the first name match), so a native
//! registered after Dyon's prelude wins even when its argument types differ.
//! The library therefore registers the colliding PureBasic commands with a
//! `str_`/`num_` prefix; these tests register the whole library and prove both
//! that the built-ins still answer and that the prefixed commands are reachable.

use vr_dyon::DyonRuntime;

/// Built-ins that a bare registration used to replace. The templated
/// `min`/`max`/`sum` loop forms share their names with the `min`/`max`
/// functions and are parsed syntactically, so exercising them guards the
/// parser path too.
const BUILTINS: &str = r#"
fn run() -> str {
    items := [1, 2, 3]
    return str(len(items)) +
        "|" + str(min([3.0, 1.0, 2.0])) +
        "|" + str(max([3.0, 1.0, 2.0])) +
        "|" + str(sum i { items[i] }) +
        "|" + str(min i { items[i] }) +
        "|" + str(max i { items[i] }) +
        "|" + str(7) +
        "|" + str(true) +
        "|" + trim("  hi  ") +
        "|" + str(abs(-3)) +
        "|" + str(round(2.5)) +
        "|" + str(floor(-1.5)) +
        "|" + str(ceil(1.2)) +
        "|" + str(sqrt(16)) +
        "|" + str(pow(2, 10)) +
        "|" + str(ln(1)) +
        "|" + str(random() < 1)
}
fn main() {}
"#;

const BUILTINS_EXPECTED: &str = "3|1|3|6|1|3|7|true|hi|3|3|-2|2|4|1024|0|true";

/// The prefixed commands the rename introduced must still be wired in.
const RENAMED: &str = r#"
fn run() -> str {
    random_seed(1)
    return num_str(str_len("héllo")) +
        "|" + str_trim("  hi  ") +
        "|" + num_str(42) +
        "|" + num_str(num_abs(-3.5)) +
        "|" + num_str(num_round(2.5)) +
        "|" + num_str(num_floor(-1.5)) +
        "|" + num_str(num_ceil(1.2)) +
        "|" + num_str(num_sqrt(9)) +
        "|" + num_str(num_pow(2, 3)) +
        "|" + num_str(num_min(3, 7)) +
        "|" + num_str(num_max(3, 7)) +
        "|" + num_str(num_ln(1)) +
        "|" + num_str(num_log10(1000)) +
        "|" + num_str(num_log2(8)) +
        "|" + num_str(num_exp(0)) +
        "|" + num_str(num_atan2(0, 1)) +
        "|" + num_str(num_sin(0)) +
        "|" + num_str(num_cos(0)) +
        "|" + num_str(num_atan(0)) +
        "|" + num_str(num_asin(0)) +
        "|" + num_str(num_acos(1)) +
        "|" + str(num_random() >= 0)
}
fn main() {}
"#;

const RENAMED_EXPECTED: &str = "5|hi|42|3.5|3|-2|2|3|8|3|7|0|3|3|1|0|0|1|0|0|0|true";

#[test]
fn dyon_builtins_survive_registration() {
    let mut runtime =
        DyonRuntime::from_source_with("builtins_end_to_end.dyon", BUILTINS, vr_std::register)
            .expect("program compiles with the native signatures");
    let result: String = runtime.call_ret("run", &[]).expect("script runs");
    assert_eq!(result, BUILTINS_EXPECTED);
}

#[test]
fn renamed_commands_are_reachable_through_dyon() {
    let mut runtime =
        DyonRuntime::from_source_with("renamed_end_to_end.dyon", RENAMED, vr_std::register)
            .expect("program compiles with the native signatures");
    let result: String = runtime.call_ret("run", &[]).expect("script runs");
    assert_eq!(result, RENAMED_EXPECTED);
}
