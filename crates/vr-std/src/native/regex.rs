//! Dyon bindings for the regular-expression commands.

use std::sync::Arc;

use dyon::{Dfn, Module, Runtime, Type, Variable};

use crate::error::RegexError;
use crate::regex;

/// Renders a value-level failure for Dyon.
fn dyon_error(error: RegexError) -> String {
    error.to_string()
}

fn regex_match(rt: &mut Runtime) -> Result<Variable, String> {
    let text: String = rt.pop()?;
    let pattern: String = rt.pop()?;
    regex::regex_match(&pattern, &text)
        .map(Variable::bool)
        .map_err(dyon_error)
}

fn regex_replace(rt: &mut Runtime) -> Result<Variable, String> {
    let text: String = rt.pop()?;
    let replacement: String = rt.pop()?;
    let pattern: String = rt.pop()?;
    regex::regex_replace(&pattern, &replacement, &text)
        .map(|value| Variable::Str(Arc::new(value)))
        .map_err(dyon_error)
}

/// Registers the regex commands into `module`.
pub(crate) fn register(module: &mut Module) {
    module.add_str(
        "regex_match",
        regex_match,
        Dfn::nl(vec![Type::Str, Type::Str], Type::Bool),
    );
    module.add_str(
        "regex_replace",
        regex_replace,
        Dfn::nl(vec![Type::Str, Type::Str, Type::Str], Type::Str),
    );
}
