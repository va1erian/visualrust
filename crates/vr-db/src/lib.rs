//! SQLite bindings exposed to Dyon.
//!
//! The crate wraps bundled `rusqlite` in two layers:
//!
//! * [`Database`] is the safe, typed API used by Rust code and tests. It owns
//!   the connection plus the rows of the last query, so a script's
//!   `database_next_row` / `database_column` pair is just cursor movement.
//! * [`register`] adds the PureBasic-style natives (`open_database`,
//!   `database_query`, `database_exec`, `database_exec_many`, `database_begin`,
//!   `database_commit`, `database_rollback`, `database_next_row`,
//!   `database_column` and `escape_string`) to a Dyon module. A caller passes it
//!   to `DyonRuntime::from_source_with`, so a Dyon program gets the commands
//!   without `vr-dyon` depending on this crate.
//!
//! BLOBs cross the Dyon boundary as arrays of whole numbers in `0..=255`, since
//! Dyon has no byte-string type; `NULL` crosses as `none()`.
//!
//! Handles are Dyon custom objects (`Arc<Mutex<Database>>`) exactly as the UI
//! bindings are, which is why the native functions take the handle by reference
//! instead of returning a copy.
//!
//! ```dyon
//! fn main() {
//!     db := open_database("data.sqlite")
//!     database_exec(db, "CREATE TABLE t (id INTEGER, name TEXT)", [])
//!     database_exec(db, "INSERT INTO t VALUES (?, ?)", [1, "Ada"])
//!     rows := database_query(db, "SELECT id, name FROM t", [])
//!     if database_next_row(db) {
//!         println(str(database_column(db, 1)))
//!     }
//! }
//! ```

#![forbid(unsafe_code)]

mod database;
mod error;
mod native;
mod value;

pub use database::Database;
pub use error::DbError;
pub use native::register;
pub use value::{DbValue, escape_string};
