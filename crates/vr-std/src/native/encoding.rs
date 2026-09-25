//! Dyon bindings for the base64 and URL encoders.
//!
//! Each command is a single string in and a single string out; the decoders
//! render their [`EncodingError`] as a Dyon error string.

use std::sync::Arc;

use dyon::{Dfn, Module, Runtime, Type, Variable};

use crate::encoding;
use crate::error::EncodingError;

/// Renders a value-level failure for Dyon.
fn dyon_error(error: EncodingError) -> String {
    error.to_string()
}

fn base64_encode(rt: &mut Runtime) -> Result<Variable, String> {
    let text: String = rt.pop()?;
    Ok(Variable::Str(Arc::new(encoding::base64_encode(&text))))
}

fn base64_decode(rt: &mut Runtime) -> Result<Variable, String> {
    let text: String = rt.pop()?;
    encoding::base64_decode(&text)
        .map(|value| Variable::Str(Arc::new(value)))
        .map_err(dyon_error)
}

fn url_encode(rt: &mut Runtime) -> Result<Variable, String> {
    let text: String = rt.pop()?;
    Ok(Variable::Str(Arc::new(encoding::url_encode(&text))))
}

fn url_decode(rt: &mut Runtime) -> Result<Variable, String> {
    let text: String = rt.pop()?;
    encoding::url_decode(&text)
        .map(|value| Variable::Str(Arc::new(value)))
        .map_err(dyon_error)
}

/// Registers the encoder commands into `module`.
pub(crate) fn register(module: &mut Module) {
    module.add_str(
        "base64_encode",
        base64_encode,
        Dfn::nl(vec![Type::Str], Type::Str),
    );
    module.add_str(
        "base64_decode",
        base64_decode,
        Dfn::nl(vec![Type::Str], Type::Str),
    );
    module.add_str(
        "url_encode",
        url_encode,
        Dfn::nl(vec![Type::Str], Type::Str),
    );
    module.add_str(
        "url_decode",
        url_decode,
        Dfn::nl(vec![Type::Str], Type::Str),
    );
}
