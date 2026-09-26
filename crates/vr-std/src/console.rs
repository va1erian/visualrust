//! Console line input.
//!
//! Dyon's `stdio` feature already provides `print`, `println`, `eprint`,
//! `eprintln` and `read_line`, so this library deliberately registers none of
//! them: shadowing them would give a script two subtly different behaviours for
//! the same name. The one command Dyon lacks is PureBasic's `Input`, which
//! prints a prompt before reading a line; [`prompt`] is its implementation.
//!
//! The reader and writer are parameters rather than `stdin`/`stdout` so the
//! behaviour is unit-tested against in-memory buffers. The Dyon native
//! ([`crate::register`]'s `input`) binds them to the real streams and is
//! therefore not deterministic; the tests cover the logic behind it instead. A
//! script that wants to read a line without a prompt can call Dyon's own
//! `read_line` directly.

use std::io::{BufRead, Write};

use crate::error::FileError;

/// Writes `message`, flushes it, then reads one line.
///
/// The trailing `\r`/`\n` is stripped so the result is the typed text, not the
/// line terminator. End of input yields an empty string rather than an error,
/// which is how a terminal behaves when the user presses Ctrl+Z.
pub fn prompt(
    input: &mut impl BufRead,
    output: &mut impl Write,
    message: &str,
) -> Result<String, FileError> {
    output
        .write_all(message.as_bytes())
        .and_then(|()| output.flush())
        .map_err(|source| stream_error("input", "<stdout>", &source))?;

    let mut line = String::new();
    input
        .read_line(&mut line)
        .map_err(|source| stream_error("input", "<stdin>", &source))?;
    Ok(line.trim_end_matches(['\r', '\n']).to_owned())
}

/// Builds an I/O error for a console stream, reusing [`FileError::Io`].
fn stream_error(function: &'static str, stream: &str, source: &std::io::Error) -> FileError {
    FileError::Io {
        function,
        path: stream.to_owned(),
        kind: source.kind(),
        message: source.to_string(),
    }
}

#[cfg(test)]
mod tests;
