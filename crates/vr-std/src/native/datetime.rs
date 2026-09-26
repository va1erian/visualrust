//! Dyon bindings for the date/time commands.
//!
//! The wall-clock commands read a [`SystemClock`] because a script has no way
//! to hand one in; the pure API takes the clock as a parameter so the tests can
//! freeze it. Each wrapper pops its arguments in reverse declaration order
//! (Dyon pushes them left to right) and renders any [`DateTimeError`] as the
//! only error a Dyon native may return, a `String`.
//!
//! All instants are UTC; see [`crate::datetime`].

use std::sync::Arc;

use dyon::{Dfn, Module, Runtime, Type, Variable};

use crate::datetime::{self, SystemClock};
use crate::error::DateTimeError;

/// Renders a value-level failure for Dyon.
fn dyon_error(error: DateTimeError) -> String {
    error.to_string()
}

fn native_date(_rt: &mut Runtime) -> Result<Variable, String> {
    Ok(Variable::f64(datetime::date(&SystemClock) as f64))
}

fn native_time(_rt: &mut Runtime) -> Result<Variable, String> {
    Ok(Variable::f64(datetime::time(&SystemClock) as f64))
}

fn native_elapsed_milliseconds(_rt: &mut Runtime) -> Result<Variable, String> {
    Ok(Variable::f64(datetime::elapsed_milliseconds(&SystemClock)))
}

fn native_format_date(rt: &mut Runtime) -> Result<Variable, String> {
    let format: String = rt.pop()?;
    let timestamp: f64 = rt.pop()?;
    let seconds = datetime::to_instant(timestamp, "format_date").map_err(dyon_error)?;
    datetime::format_date(seconds, &format)
        .map(|rendered| Variable::Str(Arc::new(rendered)))
        .map_err(dyon_error)
}

fn native_parse_date(rt: &mut Runtime) -> Result<Variable, String> {
    let format: String = rt.pop()?;
    let text: String = rt.pop()?;
    datetime::parse_date(&text, &format)
        .map(|seconds| Variable::f64(seconds as f64))
        .map_err(dyon_error)
}

fn native_delay(rt: &mut Runtime) -> Result<(), String> {
    let millis: f64 = rt.pop()?;
    let millis = datetime::to_millis(millis, "delay").map_err(dyon_error)?;
    datetime::delay(&SystemClock, millis);
    Ok(())
}

/// Registers the date/time commands into `module`.
///
/// Must run before source is loaded so Dyon's lifetime checker sees the
/// signatures.
pub(crate) fn register(module: &mut Module) {
    module.add_str("date", native_date, Dfn::nl(vec![], Type::F64));
    module.add_str("time", native_time, Dfn::nl(vec![], Type::F64));
    module.add_str(
        "elapsed_milliseconds",
        native_elapsed_milliseconds,
        Dfn::nl(vec![], Type::F64),
    );
    module.add_str(
        "format_date",
        native_format_date,
        Dfn::nl(vec![Type::F64, Type::Str], Type::Str),
    );
    module.add_str(
        "parse_date",
        native_parse_date,
        Dfn::nl(vec![Type::Str, Type::Str], Type::F64),
    );
    module.add_str("delay", native_delay, Dfn::nl(vec![Type::F64], Type::Void));
}
