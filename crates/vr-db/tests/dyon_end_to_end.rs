//! A Dyon script opens a temp database, writes and reads it back, proving the
//! natives are wired into the shared runtime.

mod common;

use common::TempDb;
use vr_dyon::DyonRuntime;

/// Expected result of the script: two rows (`rows`), two cursor steps (`count`)
/// and one row whose name is `Ada` (`matching`).
const EXPECTED: f64 = 2.0 * 100.0 + 2.0 * 10.0 + 1.0;

#[test]
fn dyon_script_round_trips_through_sqlite() {
    let temp = TempDb::new("dyon");
    // Dyon string literals do not need escaping when forward slashes are used,
    // and SQLite accepts them on Windows.
    let path = temp.path().to_string_lossy().replace('\\', "/");

    let source = format!(
        r#"
fn run() -> f64 {{
    db := open_database("{path}")
    created := database_exec(db, "CREATE TABLE people (id INTEGER, name TEXT)", [])
    first := database_exec(db, "INSERT INTO people (id, name) VALUES (?, ?)", [1, "Ada"])
    second := database_exec(db, "INSERT INTO people (id, name) VALUES (?, ?)", [2, "Bob"])
    rows := database_query(db, "SELECT id, name FROM people ORDER BY id", [])
    count := 0
    matching := 0
    loop {{
        if !database_next_row(db) {{ break }}
        count += 1
        if str(database_column(db, 1)) == "Ada" {{ matching += 1 }}
    }}
    return rows * 100 + count * 10 + matching
}}

fn main() {{}}
"#
    );

    let mut runtime = DyonRuntime::from_source("db_end_to_end.dyon", &source)
        .expect("program compiles with the native signatures");
    let result: f64 = runtime.call_ret("run", &[]).expect("script runs");

    assert_eq!(result, EXPECTED);
}

#[test]
fn escaping_a_quote_can_be_called_from_dyon() {
    let source = r#"
fn run() -> str {
    return escape_string("O'Brien")
}
fn main() {}
"#;
    let mut runtime = DyonRuntime::from_source("escape.dyon", source).expect("program compiles");
    let escaped: String = runtime.call_ret("run", &[]).expect("script runs");
    assert_eq!(escaped, "O''Brien");
}
