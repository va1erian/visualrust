//! Blob, NULL, and constraint-error mappings against a temp file.

mod common;

use common::TempDb;
use vr_db::{Database, DbError, DbValue};

#[test]
fn blob_round_trips_without_text_mangling() {
    let temp = TempDb::new("blob");
    let mut database = Database::open(temp.path()).expect("open temp database");
    database
        .exec("CREATE TABLE blobs (data BLOB)", &[])
        .expect("create table");

    // Includes NUL, the high byte, and non-UTF-8 sequences that a lossy text
    // conversion would corrupt.
    let bytes = vec![0_u8, 1, 2, 127, 128, 0xFE, 0xFF];
    database
        .exec(
            "INSERT INTO blobs VALUES (?)",
            &[DbValue::Blob(bytes.clone())],
        )
        .expect("insert blob");

    database
        .query("SELECT data FROM blobs", &[])
        .expect("select blob");
    assert!(database.next_row());
    assert_eq!(database.column(0).expect("data"), &DbValue::Blob(bytes));
}

#[test]
fn null_stays_distinct_from_zero_and_empty_text() {
    let temp = TempDb::new("null");
    let mut database = Database::open(temp.path()).expect("open temp database");
    database
        .exec("CREATE TABLE items (a, b, c)", &[])
        .expect("create table");
    database
        .exec(
            "INSERT INTO items VALUES (?, ?, ?)",
            &[
                DbValue::Null,
                DbValue::Integer(0),
                DbValue::Text(String::new()),
            ],
        )
        .expect("insert row");

    database
        .query("SELECT a, b, c FROM items", &[])
        .expect("select row");
    assert!(database.next_row());
    assert_eq!(database.column(0).expect("a"), &DbValue::Null);
    assert_eq!(database.column(1).expect("b"), &DbValue::Integer(0));
    assert_eq!(
        database.column(2).expect("c"),
        &DbValue::Text(String::new())
    );
}

#[test]
fn integer_real_and_text_keep_their_storage_class() {
    let temp = TempDb::new("storage-class");
    let mut database = Database::open(temp.path()).expect("open temp database");
    database
        .exec("CREATE TABLE t (i INTEGER, r REAL, s TEXT)", &[])
        .expect("create table");
    database
        .exec(
            "INSERT INTO t VALUES (?, ?, ?)",
            &[
                DbValue::Integer(7),
                DbValue::Real(7.5),
                DbValue::Text("7".to_owned()),
            ],
        )
        .expect("insert row");

    database
        .query("SELECT i, r, s FROM t", &[])
        .expect("select row");
    assert!(database.next_row());
    assert_eq!(database.column(0).expect("i"), &DbValue::Integer(7));
    assert_eq!(database.column(1).expect("r"), &DbValue::Real(7.5));
    assert_eq!(
        database.column(2).expect("s"),
        &DbValue::Text("7".to_owned())
    );
}

#[test]
fn unique_constraint_maps_to_a_typed_error() {
    let temp = TempDb::new("unique");
    let mut database = Database::open(temp.path()).expect("open temp database");
    database
        .exec("CREATE TABLE people (name TEXT UNIQUE)", &[])
        .expect("create table");
    database
        .exec(
            "INSERT INTO people VALUES (?)",
            &[DbValue::Text("Ada".to_owned())],
        )
        .expect("first insert");

    let error = database
        .exec(
            "INSERT INTO people VALUES (?)",
            &[DbValue::Text("Ada".to_owned())],
        )
        .expect_err("duplicate must fail");
    match error {
        DbError::UniqueConstraint { code, detail } => {
            assert_eq!(code, 2067, "SQLITE_CONSTRAINT_UNIQUE");
            assert!(detail.contains("people.name"), "detail: {detail}");
        }
        other => panic!("expected a unique violation, got {other:?}"),
    }
}

#[test]
fn primary_key_constraint_maps_to_a_typed_error() {
    let temp = TempDb::new("primary-key");
    let mut database = Database::open(temp.path()).expect("open temp database");
    database
        .exec("CREATE TABLE t (id INTEGER PRIMARY KEY)", &[])
        .expect("create table");
    database
        .exec("INSERT INTO t VALUES (?)", &[DbValue::Integer(1)])
        .expect("first insert");

    let error = database
        .exec("INSERT INTO t VALUES (?)", &[DbValue::Integer(1)])
        .expect_err("duplicate key must fail");
    match error {
        DbError::PrimaryKeyConstraint { code, .. } => {
            assert_eq!(code, 1555, "SQLITE_CONSTRAINT_PRIMARYKEY");
        }
        other => panic!("expected a primary-key violation, got {other:?}"),
    }
}

#[test]
fn not_null_constraint_maps_to_a_typed_error() {
    let temp = TempDb::new("not-null");
    let mut database = Database::open(temp.path()).expect("open temp database");
    database
        .exec("CREATE TABLE t (id INTEGER NOT NULL)", &[])
        .expect("create table");

    let error = database
        .exec("INSERT INTO t VALUES (?)", &[DbValue::Null])
        .expect_err("null must fail");
    match error {
        DbError::NotNullConstraint { code, detail } => {
            assert_eq!(code, 1299, "SQLITE_CONSTRAINT_NOTNULL");
            assert!(detail.contains("t.id"), "detail: {detail}");
        }
        other => panic!("expected a not-null violation, got {other:?}"),
    }
}
