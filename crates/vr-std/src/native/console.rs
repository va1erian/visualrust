//! Dyon binding for the console prompt.
//!
//! Dyon's own `print`, `println` and `read_line` stay untouched; the only
//! command added here is PureBasic's `Input`.

use std::io;
use std::sync::Arc;

use dyon::{Dfn, Module, Runtime, Type, Variable};

use crate::console;

/// Registers the `input` command into `module`.
pub(crate) fn register(module: &mut Module) {
    module.add_str("input", native_input, Dfn::nl(vec![Type::Str], Type::Str));
}

/// Prints the prompt and reads one line from the real terminal.
///
/// Not deterministic, so it has no end-to-end test; [`console::prompt`] is the
/// tested logic underneath.
fn native_input(rt: &mut Runtime) -> Result<Variable, String> {
    let message: String = rt.pop()?;
    let mut stdin = io::stdin().lock();
    let mut stdout = io::stdout().lock();
    console::prompt(&mut stdin, &mut stdout, &message)
        .map(|line| Variable::Str(Arc::new(line)))
        .map_err(|error| error.to_string())
}
