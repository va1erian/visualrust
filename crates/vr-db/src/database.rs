//! The owned SQLite connection and its row cursor.

use std::path::Path;

use rusqlite::Connection;
use rusqlite::params_from_iter;
use rusqlite::types::Value;

use crate::error::DbError;
use crate::value::DbValue;

/// An open SQLite database plus the rows of the last [`Database::query`].
///
/// Rows are materialised eagerly because `rusqlite::Statement` borrows its
/// `Connection`; holding a live statement next to the connection would require
/// self-referential state. Buffering also keeps every Dyon call independent,
/// which is all a sequential script needs.
pub struct Database {
    connection: Connection,
    columns: Vec<String>,
    rows: Vec<Vec<DbValue>>,
    cursor: Option<usize>,
}

impl Database {
    /// Opens the SQLite file at `path`, creating it when it does not exist.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, DbError> {
        let path = path.as_ref();
        let connection = Connection::open(path).map_err(|source| DbError::Open {
            path: path.display().to_string(),
            source,
        })?;
        Ok(Self {
            connection,
            columns: Vec::new(),
            rows: Vec::new(),
            cursor: None,
        })
    }

    /// Runs one query, buffers every row, and returns the row count.
    pub fn query(&mut self, sql: &str, parameters: &[DbValue]) -> Result<usize, DbError> {
        let values: Vec<Value> = parameters.iter().map(DbValue::to_sql).collect();
        // The statement cache reuses a compiled statement when the same SQL
        // runs again, so a script loop does not pay to re-parse its query.
        let mut statement = self.connection.prepare_cached(sql)?;
        let column_count = statement.column_count();
        self.columns = statement
            .column_names()
            .into_iter()
            .map(str::to_owned)
            .collect();
        let mapped = statement.query_map(params_from_iter(values), |row| {
            let mut values = Vec::with_capacity(column_count);
            for index in 0..column_count {
                values.push(DbValue::from_sql(row.get::<_, Value>(index)?));
            }
            Ok(values)
        })?;
        let mut rows = Vec::new();
        for row in mapped {
            rows.push(row?);
        }
        self.rows = rows;
        self.cursor = None;
        Ok(self.rows.len())
    }

    /// Runs one statement that returns no rows; returns the affected row count.
    pub fn exec(&mut self, sql: &str, parameters: &[DbValue]) -> Result<usize, DbError> {
        let values: Vec<Value> = parameters.iter().map(DbValue::to_sql).collect();
        let mut statement = self.connection.prepare_cached(sql)?;
        let affected = statement.execute(params_from_iter(values))?;
        Ok(affected)
    }

    /// Runs one statement once per parameter set, preparing it only once.
    ///
    /// This is the bulk-insert path: the statement is compiled a single time and
    /// rebound for every row, so a thousand rows cost one parse. It returns the
    /// total number of affected rows.
    pub fn exec_many(
        &mut self,
        sql: &str,
        parameter_sets: &[Vec<DbValue>],
    ) -> Result<usize, DbError> {
        let mut statement = self.connection.prepare_cached(sql)?;
        let mut affected = 0;
        for parameters in parameter_sets {
            let values: Vec<Value> = parameters.iter().map(DbValue::to_sql).collect();
            affected += statement.execute(params_from_iter(values))?;
        }
        Ok(affected)
    }

    /// Opens a transaction with `BEGIN`.
    ///
    /// SQLite refuses a nested `BEGIN`; turning that into
    /// [`DbError::TransactionAlreadyActive`] lets a script tell the mistake
    /// apart from a genuine SQL failure.
    pub fn begin(&mut self) -> Result<(), DbError> {
        if !self.connection.is_autocommit() {
            return Err(DbError::TransactionAlreadyActive);
        }
        self.connection.execute_batch("BEGIN")?;
        Ok(())
    }

    /// Commits the open transaction.
    pub fn commit(&mut self) -> Result<(), DbError> {
        if self.connection.is_autocommit() {
            return Err(DbError::NoActiveTransaction);
        }
        self.connection.execute_batch("COMMIT")?;
        Ok(())
    }

    /// Rolls back the open transaction.
    pub fn rollback(&mut self) -> Result<(), DbError> {
        if self.connection.is_autocommit() {
            return Err(DbError::NoActiveTransaction);
        }
        self.connection.execute_batch("ROLLBACK")?;
        Ok(())
    }

    /// Runs `action` inside a transaction: commit on `Ok`, roll back on `Err`.
    ///
    /// The helper exists so callers cannot forget the rollback on the error
    /// path. If the rollback itself fails, the action's error is still returned
    /// because it is the cause the caller needs to see.
    pub fn in_transaction<T>(
        &mut self,
        action: impl FnOnce(&mut Database) -> Result<T, DbError>,
    ) -> Result<T, DbError> {
        self.begin()?;
        match action(self) {
            Ok(value) => {
                self.commit()?;
                Ok(value)
            }
            Err(error) => {
                let _ = self.rollback();
                Err(error)
            }
        }
    }

    /// Advances the cursor; true while a row is available.
    ///
    /// Repeated calls after the last row keep returning false, so a script loop
    /// terminates without any extra bookkeeping.
    pub fn next_row(&mut self) -> bool {
        let next = self.cursor.map_or(0, |current| current + 1);
        if next < self.rows.len() {
            self.cursor = Some(next);
            true
        } else {
            self.cursor = Some(self.rows.len());
            false
        }
    }

    /// The value at `index` of the current row.
    pub fn column(&self, index: usize) -> Result<&DbValue, DbError> {
        let current = self.cursor.ok_or(DbError::NoCurrentRow)?;
        let row = self.rows.get(current).ok_or(DbError::NoCurrentRow)?;
        row.get(index).ok_or(DbError::ColumnOutOfRange {
            index,
            count: row.len(),
        })
    }

    /// The column names of the last query, in order.
    pub fn columns(&self) -> &[String] {
        &self.columns
    }
}
