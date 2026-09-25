//! Transactions and prepared statements against a temp file.

mod common;

use common::TempDb;
use vr_db::{Database, DbError, DbValue};

/// Opens a temp database with a single `t (id INTEGER)` table.
fn database_with_table(tag: &str) -> (TempDb, Database) {
    let temp = TempDb::new(tag);
    let mut database = Database::open(temp.path()).expect("open temp database");
    database
        .exec("CREATE TABLE t (id INTEGER)", &[])
        .expect("create table");
    (temp, database)
}

/// The number of rows in `t`; the table always exists in this file's helpers.
fn count(database: &mut Database) -> i64 {
    database
        .query("SELECT COUNT(*) FROM t", &[])
        .expect("count");
    assert!(database.next_row(), "count always returns a row");
    match database.column(0).expect("count value") {
        DbValue::Integer(value) => *value,
        other => panic!("expected an integer, got {other:?}"),
    }
}

#[test]
fn rollback_leaves_no_rows() {
    let (_temp, mut database) = database_with_table("rollback");
    database.begin().expect("begin");
    database
        .exec("INSERT INTO t VALUES (?)", &[DbValue::Integer(1)])
        .expect("insert inside transaction");
    database.rollback().expect("rollback");

    assert_eq!(count(&mut database), 0);
}

#[test]
fn commit_persists_after_reopening_the_file() {
    let temp = TempDb::new("commit");
    {
        let mut database = Database::open(temp.path()).expect("open temp database");
        database
            .exec("CREATE TABLE t (id INTEGER)", &[])
            .expect("create table");
        database.begin().expect("begin");
        database
            .exec("INSERT INTO t VALUES (?)", &[DbValue::Integer(9)])
            .expect("insert inside transaction");
        database.commit().expect("commit");
    }

    let mut reopened = Database::open(temp.path()).expect("reopen temp database");
    reopened
        .query("SELECT id FROM t", &[])
        .expect("select rows");
    assert!(reopened.next_row());
    assert_eq!(reopened.column(0).expect("id"), &DbValue::Integer(9));
}

#[test]
fn duplicate_begin_and_stray_commit_are_typed_errors() {
    let (_temp, mut database) = database_with_table("tx-state");
    database.begin().expect("first begin");

    let error = database.begin().expect_err("a nested begin must fail");
    assert!(
        matches!(error, DbError::TransactionAlreadyActive),
        "got {error:?}"
    );
    database.rollback().expect("rollback");

    let error = database.commit().expect_err("commit without a transaction");
    assert!(
        matches!(error, DbError::NoActiveTransaction),
        "got {error:?}"
    );
    let error = database
        .rollback()
        .expect_err("rollback without a transaction");
    assert!(
        matches!(error, DbError::NoActiveTransaction),
        "got {error:?}"
    );
}

#[test]
fn scoped_helper_commits_on_ok_and_rolls_back_on_err() {
    let (_temp, mut database) = database_with_table("scoped");

    database
        .in_transaction(|database| {
            database.exec("INSERT INTO t VALUES (?)", &[DbValue::Integer(1)])
        })
        .expect("the helper commits");
    assert_eq!(count(&mut database), 1);

    let error = database
        .in_transaction(|database| -> Result<(), DbError> {
            database.exec("INSERT INTO t VALUES (?)", &[DbValue::Integer(2)])?;
            Err(DbError::NoCurrentRow)
        })
        .expect_err("the action fails");
    assert!(matches!(error, DbError::NoCurrentRow), "got {error:?}");
    assert_eq!(count(&mut database), 1);
}

#[test]
fn prepared_statement_rebinds_across_many_rows() {
    let temp = TempDb::new("prepared");
    let mut database = Database::open(temp.path()).expect("open temp database");
    database
        .exec("CREATE TABLE events (id INTEGER, label TEXT)", &[])
        .expect("create table");

    let parameter_sets: Vec<Vec<DbValue>> = (0..64_i64)
        .map(|index| {
            vec![
                DbValue::Integer(index),
                DbValue::Text(format!("event-{index}")),
            ]
        })
        .collect();

    // One prepare, 64 rebinds: the statement is not recompiled per row.
    let affected = database
        .exec_many(
            "INSERT INTO events (id, label) VALUES (?, ?)",
            &parameter_sets,
        )
        .expect("insert many");
    assert_eq!(affected, 64);

    // A second run over the same SQL still reuses the cached statement.
    let more: Vec<Vec<DbValue>> = (64..128_i64)
        .map(|index| vec![DbValue::Integer(index), DbValue::Text("again".to_owned())])
        .collect();
    assert_eq!(
        database
            .exec_many("INSERT INTO events (id, label) VALUES (?, ?)", &more)
            .expect("insert many again"),
        64
    );

    database
        .query("SELECT COUNT(*) FROM events", &[])
        .expect("count");
    assert!(database.next_row());
    assert_eq!(database.column(0).expect("count"), &DbValue::Integer(128));
}
