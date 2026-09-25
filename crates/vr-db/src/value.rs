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
    /// integers widen to Dyon's single `f64` number type. Dyon has no byte-string
    /// type, so a BLOB crosses as an array of whole numbers in `0..=255`.
    pub fn to_dyon(&self) -> Variable {
        match self {
            Self::Null => Variable::Option(None),
            Self::Integer(number) => Variable::f64(*number as f64),
            Self::Real(number) => Variable::f64(*number),
            Self::Text(text) => Variable::Str(Arc::new(text.clone())),
            Self::Blob(bytes) => blob_to_dyon(bytes),
        }
    }

    /// Reads one SQL parameter from a Dyon value.
    ///
    /// Dyon has a single number type, so a whole finite number binds as an
    /// integer and everything else binds as a real; `none()` binds as `NULL`,
    /// `some(x)` is unwrapped, and an array of whole numbers in `0..=255` binds
    /// as a BLOB (the inverse of what [`DbValue::to_dyon`] emits).
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
            Variable::Array(items) => read_blob(rt, items),
            other => Err(DbError::UnsupportedParameter(
                other.typeof_var().to_string(),
            )),
        }
    }
}

/// Wraps bytes as Dyon's array type.
///
/// Dyon defines `Array` as `Arc<Vec<Variable>>` and `Variable` is not `Sync`, so
/// clippy's non-`Send`/`Sync` warning is unavoidable when matching the Dyon API.
#[allow(clippy::arc_with_non_send_sync)]
fn blob_to_dyon(bytes: &[u8]) -> Variable {
    Variable::Array(Arc::new(
        bytes
            .iter()
            .map(|byte| Variable::f64(f64::from(*byte)))
            .collect(),
    ))
}

/// Converts a Dyon array into bytes, rejecting anything that is not a whole
/// number in `0..=255` so a typo cannot silently truncate a blob.
fn read_blob(rt: &Runtime, items: &dyon::Array) -> Result<DbValue, DbError> {
    let mut bytes = Vec::with_capacity(items.len());
    for item in items.iter() {
        match rt.get(item) {
            Variable::F64(number, _) if is_byte(*number) => bytes.push(*number as u8),
            other => {
                return Err(DbError::InvalidBlobElement(other.typeof_var().to_string()));
            }
        }
    }
    Ok(DbValue::Blob(bytes))
}

/// Whether a Dyon number is a byte value (`0..=255`, no fraction).
fn is_byte(number: f64) -> bool {
    number.is_finite() && number.fract() == 0.0 && (0.0..=255.0).contains(&number)
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
