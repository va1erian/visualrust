//! Dyon bindings for the digests and HMAC.

use std::sync::Arc;

use dyon::{Dfn, Module, Runtime, Type, Variable};

use crate::error::HashError;
use crate::hash;

/// Renders a value-level failure for Dyon.
fn dyon_error(error: HashError) -> String {
    error.to_string()
}

/// Wraps a hex digest as a Dyon string.
fn hex_variable(value: String) -> Variable {
    Variable::Str(Arc::new(value))
}

fn md5(rt: &mut Runtime) -> Result<Variable, String> {
    let text: String = rt.pop()?;
    Ok(hex_variable(hash::md5(&text)))
}

fn sha1(rt: &mut Runtime) -> Result<Variable, String> {
    let text: String = rt.pop()?;
    Ok(hex_variable(hash::sha1(&text)))
}

fn sha256(rt: &mut Runtime) -> Result<Variable, String> {
    let text: String = rt.pop()?;
    Ok(hex_variable(hash::sha256(&text)))
}

fn hmac_sha256(rt: &mut Runtime) -> Result<Variable, String> {
    let text: String = rt.pop()?;
    let key: String = rt.pop()?;
    hash::hmac_sha256(&key, &text)
        .map(hex_variable)
        .map_err(dyon_error)
}

/// Registers the hashing commands into `module`.
pub(crate) fn register(module: &mut Module) {
    module.add_str("md5", md5, Dfn::nl(vec![Type::Str], Type::Str));
    module.add_str("sha1", sha1, Dfn::nl(vec![Type::Str], Type::Str));
    module.add_str("sha256", sha256, Dfn::nl(vec![Type::Str], Type::Str));
    module.add_str(
        "hmac_sha256",
        hmac_sha256,
        Dfn::nl(vec![Type::Str, Type::Str], Type::Str),
    );
}
