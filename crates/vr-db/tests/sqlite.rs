//! Typed round trips through the `Database` API against a temp file.

mod common;

use common::TempDb;
use vr_db::{Database, DbError, DbValue, escape_string};

#[test]
fn stores_and_reads_back_every_supported_type() {
    let temp = TempDb::new("typed");
    let mut database = Database::open(temp.path()).expect("open temp database");
    database
        .exec(
            "CREATE TABLE items (id INTEGER, label TEXT, weight REAL, note TEXT)",
            &[],
        )
        .expect("create table");

    let inserted = database
        .exec(
            "INSERT INTO items VALUES (?, ?, ?, ?)",
            &[
                DbValue::Integer(7),
                DbValue::Text("Ada".to_owned()),
                DbValue::Real(9.5),
                DbValue::Null,
            ],
        )
        .expect("insert row");
    assert_eq!(inserted, 1);

    let rows = database
        .query("SELECT id, label, weight, note FROM items", &[])
        .expect("select rows");
    assert_eq!(rows, 1);
    assert_eq!(database.columns().len(), 4);
    assert_eq!(database.columns()[1], "label");

    assert!(database.next_row());
    assert_eq!(database.column(0).expect("id"), &DbValue::Integer(7));
    assert_eq!(
        database.column(1).expect("label"),
        &DbValue::Text("Ada".to_owned())
    );
    assert_eq!(database.column(2).expect("weight"), &DbValue::Real(9.5));
    assert_eq!(database.column(3).expect("note"), &DbValue::Null);
}

#[test]
fn cursor_walks_rows_in_order_and_then_stops() {
    let temp = TempDb::new("cursor");
    let mut database = Database::open(temp.path()).expect("open temp database");
    database
        .exec("CREATE TABLE numbers (value INTEGER)", &[])
        .expect("create table");
    for value in 1..=3 {
        database
            .exec("INSERT INTO numbers VALUES (?)", &[DbValue::Integer(value)])
            .expect("insert row");
    }

    database
        .query("SELECT value FROM numbers ORDER BY value", &[])
        .expect("select rows");

    for expected in 1..=3 {
        assert!(database.next_row(), "row {expected} is present");
        assert_eq!(
            database.column(0).expect("value"),
            &DbValue::Integer(expected)
        );
    }
    // Past the end the cursor stays false instead of wrapping.
    assert!(!database.next_row());
    assert!(!database.next_row());
}

#[test]
fn parameters_filter_rows() {
    let temp = TempDb::new("params");
    let mut database = Database::open(temp.path()).expect("open temp database");
    database
        .exec("CREATE TABLE people (name TEXT, score REAL)", &[])
        .expect("create table");
    for (name, score) in [("Ada", 9.5_f64), ("Bob", 7.0_f64)] {
        database
            .exec(
                "INSERT INTO people VALUES (?, ?)",
                &[DbValue::Text(name.to_owned()), DbValue::Real(score)],
            )
            .expect("insert row");
    }

    let rows = database
        .query(
            "SELECT score FROM people WHERE name = ?",
            &[DbValue::Text("Ada".to_owned())],
        )
        .expect("select filtered");
    assert_eq!(rows, 1);
    assert!(database.next_row());
    assert_eq!(database.column(0).expect("score"), &DbValue::Real(9.5));
}

#[test]
fn reading_without_a_current_row_is_typed() {
    let temp = TempDb::new("no-row");
    let mut database = Database::open(temp.path()).expect("open temp database");
    database.query("SELECT 1", &[]).expect("select constant");

    let error = database.column(0).expect_err("no row selected yet");
    assert!(matches!(error, DbError::NoCurrentRow), "got {error:?}");
}

#[test]
fn reading_a_missing_column_is_typed() {
    let temp = TempDb::new("bad-column");
    let mut database = Database::open(temp.path()).expect("open temp database");
    database.query("SELECT 1", &[]).expect("select constant");
    assert!(database.next_row());

    let error = database.column(3).expect_err("column is absent");
    assert!(
        matches!(error, DbError::ColumnOutOfRange { index: 3, count: 1 }),
        "got {error:?}"
    );
}

#[test]
fn escaping_a_quote_round_trips_through_a_literal() {
    let temp = TempDb::new("escape");
    let mut database = Database::open(temp.path()).expect("open temp database");
    database
        .exec("CREATE TABLE notes (body TEXT)", &[])
        .expect("create table");

    let literal = format!("INSERT INTO notes VALUES ('{}')", escape_string("O'Brien"));
    database.exec(&literal, &[]).expect("insert literal");

    database
        .query("SELECT body FROM notes", &[])
        .expect("select rows");
    assert!(database.next_row());
    assert_eq!(
        database.column(0).expect("body"),
        &DbValue::Text("O'Brien".to_owned())
    );
}
