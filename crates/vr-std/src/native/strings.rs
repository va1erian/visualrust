//! Dyon bindings for the string library.
//!
//! Every wrapper pops its arguments in reverse declaration order (Dyon pushes
//! them left to right), renders any [`StringError`] into the only error type a
//! Dyon native may return, and yields a [`Variable`]. Argument-type mistakes
//! (a non-string, a missing argument) are already reported by `rt.pop`, so the
//! wrappers only add the value-level checks in [`crate::strings`].

use std::sync::Arc;

use dyon::{Dfn, Module, Runtime, Type, Variable};

use crate::error::StringError;
use crate::strings;

/// Renders a value-level failure for Dyon.
fn dyon_error(error: StringError) -> String {
    error.to_string()
}

/// Pops a numeric position/length argument and validates it.
fn pop_index(rt: &mut Runtime, function: &'static str) -> Result<i64, String> {
    let value: f64 = rt.pop()?;
    strings::to_index(value, function).map_err(dyon_error)
}

fn text_variable(value: String) -> Variable {
    Variable::Str(Arc::new(value))
}

fn native_left(rt: &mut Runtime) -> Result<Variable, String> {
    let count = pop_index(rt, "left")?;
    let text: String = rt.pop()?;
    strings::left(&text, count)
        .map(text_variable)
        .map_err(dyon_error)
}

fn native_right(rt: &mut Runtime) -> Result<Variable, String> {
    let count = pop_index(rt, "right")?;
    let text: String = rt.pop()?;
    strings::right(&text, count)
        .map(text_variable)
        .map_err(dyon_error)
}

fn native_mid(rt: &mut Runtime) -> Result<Variable, String> {
    let length = pop_index(rt, "mid")?;
    let start = pop_index(rt, "mid")?;
    let text: String = rt.pop()?;
    strings::mid(&text, start, length)
        .map(text_variable)
        .map_err(dyon_error)
}

fn native_len(rt: &mut Runtime) -> Result<Variable, String> {
    let text: String = rt.pop()?;
    Ok(Variable::f64(strings::len(&text) as f64))
}

fn native_ucase(rt: &mut Runtime) -> Result<Variable, String> {
    let text: String = rt.pop()?;
    Ok(text_variable(strings::ucase(&text)))
}

fn native_lcase(rt: &mut Runtime) -> Result<Variable, String> {
    let text: String = rt.pop()?;
    Ok(text_variable(strings::lcase(&text)))
}

fn native_find_string(rt: &mut Runtime) -> Result<Variable, String> {
    let needle: String = rt.pop()?;
    let text: String = rt.pop()?;
    Ok(Variable::f64(strings::find_string(&text, &needle) as f64))
}

fn native_replace_string(rt: &mut Runtime) -> Result<Variable, String> {
    let replacement: String = rt.pop()?;
    let search: String = rt.pop()?;
    let text: String = rt.pop()?;
    Ok(text_variable(strings::replace_string(
        &text,
        &search,
        &replacement,
    )))
}

fn native_trim(rt: &mut Runtime) -> Result<Variable, String> {
    let text: String = rt.pop()?;
    Ok(text_variable(strings::trim(&text)))
}

fn native_ltrim(rt: &mut Runtime) -> Result<Variable, String> {
    let text: String = rt.pop()?;
    Ok(text_variable(strings::ltrim(&text)))
}

fn native_rtrim(rt: &mut Runtime) -> Result<Variable, String> {
    let text: String = rt.pop()?;
    Ok(text_variable(strings::rtrim(&text)))
}

fn native_count_string(rt: &mut Runtime) -> Result<Variable, String> {
    let needle: String = rt.pop()?;
    let text: String = rt.pop()?;
    Ok(Variable::f64(strings::count_string(&text, &needle) as f64))
}

fn native_insert_string(rt: &mut Runtime) -> Result<Variable, String> {
    let insert: String = rt.pop()?;
    let position = pop_index(rt, "insert_string")?;
    let text: String = rt.pop()?;
    strings::insert_string(&text, position, &insert)
        .map(text_variable)
        .map_err(dyon_error)
}

fn native_remove_string(rt: &mut Runtime) -> Result<Variable, String> {
    let length = pop_index(rt, "remove_string")?;
    let start = pop_index(rt, "remove_string")?;
    let text: String = rt.pop()?;
    strings::remove_string(&text, start, length)
        .map(text_variable)
        .map_err(dyon_error)
}

fn native_string_field(rt: &mut Runtime) -> Result<Variable, String> {
    let delimiter: String = rt.pop()?;
    let index = pop_index(rt, "string_field")?;
    let text: String = rt.pop()?;
    strings::string_field(&text, index, &delimiter)
        .map(text_variable)
        .map_err(dyon_error)
}

fn native_space(rt: &mut Runtime) -> Result<Variable, String> {
    let count = pop_index(rt, "space")?;
    strings::space(count).map(text_variable).map_err(dyon_error)
}

fn native_val(rt: &mut Runtime) -> Result<Variable, String> {
    let text: String = rt.pop()?;
    Ok(Variable::f64(strings::val(&text)))
}

fn native_str(rt: &mut Runtime) -> Result<Variable, String> {
    let value: f64 = rt.pop()?;
    Ok(text_variable(strings::number_to_string(value)))
}

fn native_hex(rt: &mut Runtime) -> Result<Variable, String> {
    let digits = pop_index(rt, "hex")?;
    let value: f64 = rt.pop()?;
    strings::hex(value, digits)
        .map(text_variable)
        .map_err(dyon_error)
}

fn native_bin(rt: &mut Runtime) -> Result<Variable, String> {
    let digits = pop_index(rt, "bin")?;
    let value: f64 = rt.pop()?;
    strings::bin(value, digits)
        .map(text_variable)
        .map_err(dyon_error)
}

fn native_format(rt: &mut Runtime) -> Result<Variable, String> {
    let digits = pop_index(rt, "format")?;
    let value: f64 = rt.pop()?;
    strings::format(value, digits)
        .map(text_variable)
        .map_err(dyon_error)
}

fn native_lset(rt: &mut Runtime) -> Result<Variable, String> {
    let pad: String = rt.pop()?;
    let length = pop_index(rt, "lset")?;
    let text: String = rt.pop()?;
    strings::lset(&text, length, &pad)
        .map(text_variable)
        .map_err(dyon_error)
}

fn native_rset(rt: &mut Runtime) -> Result<Variable, String> {
    let pad: String = rt.pop()?;
    let length = pop_index(rt, "rset")?;
    let text: String = rt.pop()?;
    strings::rset(&text, length, &pad)
        .map(text_variable)
        .map_err(dyon_error)
}

/// Registers the string commands into `module`.
///
/// Must run before source is loaded so Dyon's lifetime checker sees the
/// signatures. Dyon resolves a call by name only, so a command sharing a name
/// with a built-in would shadow it for every later script (`len(array)`,
/// `str(any)`, `trim(str)`). The three that would collide register as
/// `str_len`, `num_str` and `str_trim`; the rest keep their PureBasic names.
pub(crate) fn register(module: &mut Module) {
    module.add_str(
        "left",
        native_left,
        Dfn::nl(vec![Type::Str, Type::F64], Type::Str),
    );
    module.add_str(
        "right",
        native_right,
        Dfn::nl(vec![Type::Str, Type::F64], Type::Str),
    );
    module.add_str(
        "mid",
        native_mid,
        Dfn::nl(vec![Type::Str, Type::F64, Type::F64], Type::Str),
    );
    module.add_str("str_len", native_len, Dfn::nl(vec![Type::Str], Type::F64));
    module.add_str("ucase", native_ucase, Dfn::nl(vec![Type::Str], Type::Str));
    module.add_str("lcase", native_lcase, Dfn::nl(vec![Type::Str], Type::Str));
    module.add_str(
        "find_string",
        native_find_string,
        Dfn::nl(vec![Type::Str, Type::Str], Type::F64),
    );
    module.add_str(
        "replace_string",
        native_replace_string,
        Dfn::nl(vec![Type::Str, Type::Str, Type::Str], Type::Str),
    );
    module.add_str("str_trim", native_trim, Dfn::nl(vec![Type::Str], Type::Str));
    module.add_str("ltrim", native_ltrim, Dfn::nl(vec![Type::Str], Type::Str));
    module.add_str("rtrim", native_rtrim, Dfn::nl(vec![Type::Str], Type::Str));
    module.add_str(
        "count_string",
        native_count_string,
        Dfn::nl(vec![Type::Str, Type::Str], Type::F64),
    );
    module.add_str(
        "insert_string",
        native_insert_string,
        Dfn::nl(vec![Type::Str, Type::F64, Type::Str], Type::Str),
    );
    module.add_str(
        "remove_string",
        native_remove_string,
        Dfn::nl(vec![Type::Str, Type::F64, Type::F64], Type::Str),
    );
    module.add_str(
        "string_field",
        native_string_field,
        Dfn::nl(vec![Type::Str, Type::F64, Type::Str], Type::Str),
    );
    module.add_str("space", native_space, Dfn::nl(vec![Type::F64], Type::Str));
    module.add_str("val", native_val, Dfn::nl(vec![Type::Str], Type::F64));
    module.add_str("num_str", native_str, Dfn::nl(vec![Type::F64], Type::Str));
    module.add_str(
        "hex",
        native_hex,
        Dfn::nl(vec![Type::F64, Type::F64], Type::Str),
    );
    module.add_str(
        "bin",
        native_bin,
        Dfn::nl(vec![Type::F64, Type::F64], Type::Str),
    );
    module.add_str(
        "format",
        native_format,
        Dfn::nl(vec![Type::F64, Type::F64], Type::Str),
    );
    module.add_str(
        "lset",
        native_lset,
        Dfn::nl(vec![Type::Str, Type::F64, Type::Str], Type::Str),
    );
    module.add_str(
        "rset",
        native_rset,
        Dfn::nl(vec![Type::Str, Type::F64, Type::Str], Type::Str),
    );
}
