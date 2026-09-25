//! Typed errors for the SQLite binding.
//!
//! Rusqlite reports failures as its own `Error`; the extra variants here describe
//! mistakes that only make sense at the Dyon boundary (a bad handle, a
//! non-array parameter list, a column read before a row was selected).

use thiserror::Error;

/// Anything that can go wrong while using a [`Database`](crate::Database).
#[derive(Debug, Error)]
pub enum DbError {
    /// The database file could not be opened (missing directory, permissions,
    /// or a corrupt file).
    #[error("could not open database `{path}`: {source}")]
    Open {
        path: String,
        #[source]
        source: rusqlite::Error,
    },
    /// SQLite rejected the statement, a step failed, or a row did not match the
    /// requested type.
    #[error("database error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    /// A previous panic left a handle's mutex unusable.
    #[error("the database handle was poisoned by an earlier panic")]
    Poisoned,
    /// A Dyon value that should have been a handle was some other type.
    #[error("value is not a database handle")]
    NotAHandle,
    /// The parameters argument was not an array.
    #[error("expected an array of SQL parameters, got {0}")]
    NotAnArray(String),
    /// A parameter had a type SQLite cannot bind.
    #[error("cannot bind a `{0}` as an SQL parameter")]
    UnsupportedParameter(String),
    /// `database_column` ran before `database_next_row` selected a row.
    #[error("no row is current; call database_next_row first")]
    NoCurrentRow,
    /// The column does not exist on the current row.
    #[error("column index {index} is out of range (the row has {count} columns)")]
    ColumnOutOfRange { index: usize, count: usize },
    /// A negative or fractional index cannot name a column.
    #[error("column index {0} is not a valid non-negative integer")]
    InvalidColumnIndex(f64),
}
