//! The typed value that crosses the Dyon/SQLite boundary.
//!
//! Keeping a dedicated enum means a script cannot confuse `1`, `"1"` and a
//! missing value, and the Rust API does not leak `rusqlite::types::Value` into
//! its public surface.

use std::sync::Arc;

use dyon::{Runtime, Variable};
use rusqlite::types::Value;

use crate::error::DbError;

/// One SQLite value, owned by Rust.
#[derive(Clone, Debug, PartialEq)]
pub enum DbValue {
    /// SQL `NULL`.
    Null,
    /// Signed 64-bit integer.
    Integer(i64),
    /// 64-bit floating point number.
    Real(f64),
    /// UTF-8 text.
    Text(String),
    /// Raw bytes (SQL `BLOB`).
    Blob(Vec<u8>),
}

impl DbValue {
    /// Converts a value read from a row into the crate's typed value.
    pub(crate) fn from_sql(value: Value) -> Self {
        match value {
            Value::Null => Self::Null,
            Value::Integer(number) => Self::Integer(number),
            Value::Real(number) => Self::Real(number),
            Value::Text(text) => Self::Text(text),
            Value::Blob(bytes) => Self::Blob(bytes),
        }
    }

    /// Converts the typed value into rusqlite's binding representation.
    pub(crate) fn to_sql(&self) -> Value {
        match self {
            Self::Null => Value::Null,
            Self::Integer(number) => Value::Integer(*number),
            Self::Real(number) => Value::Real(*number),
            Self::Text(text) => Value::Text(text.clone()),
            Self::Blob(bytes) => Value::Blob(bytes.clone()),
        }
    }

    /// Renders the value for Dyon.
    ///
    /// `NULL` becomes an option so a script can tell it apart from `0` and `""`;
    /// integers widen to Dyon's single `f64` number type.
    pub fn to_dyon(&self) -> Variable {
        match self {
            Self::Null => Variable::Option(None),
            Self::Integer(number) => Variable::f64(*number as f64),
            Self::Real(number) => Variable::f64(*number),
            Self::Text(text) => Variable::Str(Arc::new(text.clone())),
            // Dyon has no byte-string type, so a lossy text view is the most it
            // can carry; it matches what a script would print anyway.
            Self::Blob(bytes) => {
                Variable::Str(Arc::new(String::from_utf8_lossy(bytes).into_owned()))
            }
        }
    }

    /// Reads one SQL parameter from a Dyon value.
    ///
    /// Dyon has a single number type, so a whole finite number binds as an
    /// integer and everything else binds as a real; `none()` binds as `NULL` and
    /// `some(x)` is unwrapped.
    pub(crate) fn from_dyon(rt: &Runtime, variable: &Variable) -> Result<Self, DbError> {
        match rt.get(variable) {
            Variable::F64(number, _) if binds_as_integer(*number) => {
                Ok(Self::Integer(*number as i64))
            }
            Variable::F64(number, _) => Ok(Self::Real(*number)),
            Variable::Str(text) => Ok(Self::Text((**text).clone())),
            Variable::Bool(value, _) => Ok(Self::Integer(i64::from(*value))),
            Variable::Option(None) => Ok(Self::Null),
            Variable::Option(Some(inner)) => Self::from_dyon(rt, inner),
            other => Err(DbError::UnsupportedParameter(
                other.typeof_var().to_string(),
            )),
        }
    }
}

/// Whether a Dyon number is exactly representable as an SQLite integer.
fn binds_as_integer(number: f64) -> bool {
    number.is_finite()
        && number.fract() == 0.0
        && number >= i64::MIN as f64
        && number <= i64::MAX as f64
}

/// Escapes `input` so it can sit inside a single-quoted SQL string literal.
///
/// SQLite escapes a quote by doubling it; that is the only rule, so no backslash
/// handling is added (doing so would corrupt values on the way back out).
pub fn escape_string(input: &str) -> String {
    input.replace('\'', "''")
}
