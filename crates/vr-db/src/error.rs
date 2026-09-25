//! Typed errors for the SQLite binding.
//!
//! Rusqlite reports failures as its own `Error`; the extra variants here describe
//! mistakes that only make sense at the Dyon boundary (a bad handle, a
//! non-array parameter list, a column read before a row was selected). SQLite
//! constraint failures get one variant each so a caller can branch on the cause
//! instead of parsing a message string.

use thiserror::Error;

/// Extended result codes for the SQLite constraint failures a script is most
/// likely to hit. They are stable SQLite ABI values; naming them locally keeps
/// `libsqlite3-sys` out of this crate's dependency list.
const SQLITE_CONSTRAINT: i32 = 19;
const SQLITE_CONSTRAINT_CHECK: i32 = 275;
const SQLITE_CONSTRAINT_FOREIGNKEY: i32 = 787;
const SQLITE_CONSTRAINT_NOTNULL: i32 = 1299;
const SQLITE_CONSTRAINT_PRIMARYKEY: i32 = 1555;
const SQLITE_CONSTRAINT_UNIQUE: i32 = 2067;

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
    /// Any SQLite failure without a more specific variant: syntax errors, a
    /// missing table, an I/O error. `code` is the extended result code, or `0`
    /// when the failure did not come from SQLite itself.
    #[error("SQLite error {code}: {message}")]
    Sqlite { code: i32, message: String },
    /// A `UNIQUE` index rejected the row.
    #[error("UNIQUE constraint failed: {detail} (SQLite code {code})")]
    UniqueConstraint { code: i32, detail: String },
    /// A `PRIMARY KEY` rejected the row.
    #[error("PRIMARY KEY constraint failed: {detail} (SQLite code {code})")]
    PrimaryKeyConstraint { code: i32, detail: String },
    /// A `NOT NULL` column rejected a `NULL`.
    #[error("NOT NULL constraint failed: {detail} (SQLite code {code})")]
    NotNullConstraint { code: i32, detail: String },
    /// A foreign-key relationship rejected the row.
    #[error("FOREIGN KEY constraint failed (SQLite code {code})")]
    ForeignKeyConstraint { code: i32 },
    /// A `CHECK` expression rejected the row.
    #[error("CHECK constraint failed: {detail} (SQLite code {code})")]
    CheckConstraint { code: i32, detail: String },
    /// A constraint failure SQLite reports with no dedicated variant.
    #[error("SQLite constraint failed: {detail} (SQLite code {code})")]
    Constraint { code: i32, detail: String },
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
    /// A BLOB parameter was not an array of whole numbers in `0..=255`.
    #[error("a BLOB parameter must be an array of whole numbers 0-255, got `{0}`")]
    InvalidBlobElement(String),
    /// `database_column` ran before `database_next_row` selected a row.
    #[error("no row is current; call database_next_row first")]
    NoCurrentRow,
    /// The column does not exist on the current row.
    #[error("column index {index} is out of range (the row has {count} columns)")]
    ColumnOutOfRange { index: usize, count: usize },
    /// A negative or fractional index cannot name a column.
    #[error("column index {0} is not a valid non-negative integer")]
    InvalidColumnIndex(f64),
    /// `begin` ran while a transaction was already open.
    #[error("a transaction is already active on this database")]
    TransactionAlreadyActive,
    /// `commit` or `rollback` ran with no open transaction.
    #[error("no transaction is active on this database")]
    NoActiveTransaction,
}

impl From<rusqlite::Error> for DbError {
    fn from(source: rusqlite::Error) -> Self {
        // Only `SqliteFailure` carries a result code; everything else (a type
        // conversion, a bad parameter count) is surfaced with code `0`.
        let rusqlite::Error::SqliteFailure(error, detail) = &source else {
            return Self::Sqlite {
                code: 0,
                message: source.to_string(),
            };
        };
        let code = error.extended_code;
        let detail = detail
            .clone()
            .unwrap_or_else(|| format!("SQLite error {code}"));
        match code {
            SQLITE_CONSTRAINT_UNIQUE => Self::UniqueConstraint { code, detail },
            SQLITE_CONSTRAINT_PRIMARYKEY => Self::PrimaryKeyConstraint { code, detail },
            SQLITE_CONSTRAINT_NOTNULL => Self::NotNullConstraint { code, detail },
            SQLITE_CONSTRAINT_FOREIGNKEY => Self::ForeignKeyConstraint { code },
            SQLITE_CONSTRAINT_CHECK => Self::CheckConstraint { code, detail },
            // The extended code is `base | (n << 8)`, so masking the low byte
            // recognises every member of the SQLITE_CONSTRAINT family.
            _ if code & 0xFF == SQLITE_CONSTRAINT => Self::Constraint { code, detail },
            _ => Self::Sqlite {
                code,
                message: detail,
            },
        }
    }
}
