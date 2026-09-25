//! Dyon native functions for the SQLite binding.
//!
//! Every fallible step becomes a `String` here: that is the only error Dyon
//! native functions can return, so [`DbError`] is rendered once and then
//! surfaced by the runtime as a normal Dyon error.

use std::sync::Arc;

use dyon::embed::to_rust_object;
use dyon::{Dfn, Module, Runtime, RustObject, Type, Variable};

use crate::database::Database;
use crate::error::DbError;
use crate::value::{DbValue, escape_string as escape};

/// Registers the database commands into `module`.
///
/// Must run before the source is loaded so Dyon's lifetime checker sees the
/// signatures.
pub fn register(module: &mut Module) {
    let database = ad_hoc("Database");
    module.add_str(
        "open_database",
        open_database,
        Dfn::nl(vec![Type::Str], database.clone()),
    );
    module.add_str(
        "database_query",
        database_query,
        Dfn::nl(
            vec![
                database.clone(),
                Type::Str,
                Type::Array(Box::new(Type::Any)),
            ],
            Type::F64,
        ),
    );
    module.add_str(
        "database_exec",
        database_exec,
        Dfn::nl(
            vec![
                database.clone(),
                Type::Str,
                Type::Array(Box::new(Type::Any)),
            ],
            Type::F64,
        ),
    );
    module.add_str(
        "database_next_row",
        database_next_row,
        Dfn::nl(vec![database.clone()], Type::Bool),
    );
    module.add_str(
        "database_column",
        database_column,
        Dfn::nl(vec![database, Type::F64], Type::Any),
    );
    module.add_str(
        "escape_string",
        escape_string,
        Dfn::nl(vec![Type::Str], Type::Str),
    );
}

/// An ad-hoc type named `name`, so a handle returned by `open_database` is
/// accepted only by the database commands.
fn ad_hoc(name: &str) -> Type {
    Type::AdHoc(Arc::new(name.to_owned()), Box::new(Type::Any))
}

fn dyon_error(error: DbError) -> String {
    error.to_string()
}

fn open_database(rt: &mut Runtime) -> Result<Variable, String> {
    let path: String = rt.pop()?;
    let database = Database::open(&path).map_err(dyon_error)?;
    Ok(Variable::RustObject(to_rust_object(database)))
}

fn database_query(rt: &mut Runtime) -> Result<Variable, String> {
    let raw: Variable = rt.pop()?;
    let sql: String = rt.pop()?;
    let object: RustObject = rt.pop()?;
    let parameters = read_parameters(rt, &raw)?;
    let count = database_mut(&object, |database| database.query(&sql, &parameters))?;
    Ok(Variable::f64(count as f64))
}

fn database_exec(rt: &mut Runtime) -> Result<Variable, String> {
    let raw: Variable = rt.pop()?;
    let sql: String = rt.pop()?;
    let object: RustObject = rt.pop()?;
    let parameters = read_parameters(rt, &raw)?;
    let affected = database_mut(&object, |database| database.exec(&sql, &parameters))?;
    Ok(Variable::f64(affected as f64))
}

fn database_next_row(rt: &mut Runtime) -> Result<Variable, String> {
    let object: RustObject = rt.pop()?;
    let advanced = database_mut(&object, |database| Ok(database.next_row()))?;
    Ok(Variable::bool(advanced))
}

fn database_column(rt: &mut Runtime) -> Result<Variable, String> {
    let index: f64 = rt.pop()?;
    let object: RustObject = rt.pop()?;
    let index = column_index(index)?;
    let value = database_mut(&object, |database| database.column(index).cloned())?;
    Ok(value.to_dyon())
}

fn escape_string(rt: &mut Runtime) -> Result<Variable, String> {
    let input: String = rt.pop()?;
    Ok(Variable::Str(Arc::new(escape(&input))))
}

/// Reads the parameter array, resolving any references through `rt`.
fn read_parameters(rt: &Runtime, variable: &Variable) -> Result<Vec<DbValue>, String> {
    let resolved = rt.get(variable);
    let Variable::Array(items) = resolved else {
        return Err(DbError::NotAnArray(resolved.typeof_var().to_string()).to_string());
    };
    items
        .iter()
        .map(|item| DbValue::from_dyon(rt, item).map_err(dyon_error))
        .collect()
}

/// Runs `action` with exclusive access to the handle, mapping lock and
/// downcast failures to Dyon errors.
fn database_mut<T>(
    object: &RustObject,
    action: impl FnOnce(&mut Database) -> Result<T, DbError>,
) -> Result<T, String> {
    let mut guard = object.lock().map_err(|_| DbError::Poisoned.to_string())?;
    let database = guard
        .downcast_mut::<Database>()
        .ok_or_else(|| DbError::NotAHandle.to_string())?;
    action(database).map_err(dyon_error)
}

/// Validates a Dyon column index before it becomes a `usize`.
fn column_index(index: f64) -> Result<usize, String> {
    if index.is_finite() && index.fract() == 0.0 && index >= 0.0 && index <= usize::MAX as f64 {
        Ok(index as usize)
    } else {
        Err(DbError::InvalidColumnIndex(index).to_string())
    }
}
