//! Typed mapping of Dyon's textual errors.
//!
//! Dyon exposes failures only as preformatted `String`s, so the source
//! position is recovered by parsing the `line,column:` marker that
//! `piston_meta`'s error writer emits. If Dyon ever exposes a structured
//! error type, that should replace this parser.

use thiserror::Error;

/// A one-based source location inside a Dyon program.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourcePosition {
    pub line: u32,
    pub column: u32,
}

/// Anything that can go wrong while loading or running a Dyon program.
#[derive(Debug, Error)]
pub enum DyonError {
    /// The source failed to parse, pass the lifetime checker, or convert to an
    /// AST.
    #[error("dyon compile error in `{source_name}`: {message}")]
    Compile {
        source_name: String,
        message: String,
        position: Option<SourcePosition>,
    },
    /// The program loaded but failed while executing.
    #[error("dyon runtime error: {message}")]
    Runtime {
        message: String,
        position: Option<SourcePosition>,
    },
    /// The source file could not be read.
    #[error("failed to read `{path}`: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    /// The program asked for a window but this session cannot create one (a
    /// headless CI runner, or Windows Sandbox without a desktop). Callers treat
    /// this as a skip rather than a failure.
    #[error("no interactive window could be created")]
    NoWindow,
}

impl DyonError {
    /// The offending source position, when Dyon's text carried one.
    pub fn position(&self) -> Option<SourcePosition> {
        match self {
            DyonError::Compile { position, .. } | DyonError::Runtime { position, .. } => *position,
            DyonError::Io { .. } | DyonError::NoWindow => None,
        }
    }

    pub(crate) fn compile(source_name: &str, message: String) -> Self {
        Self::Compile {
            source_name: source_name.to_owned(),
            position: parse_position(&message),
            message,
        }
    }

    pub(crate) fn runtime(message: String) -> Self {
        Self::Runtime {
            position: parse_position(&message),
            message,
        }
    }
}

fn parse_position(message: &str) -> Option<SourcePosition> {
    message.lines().find_map(parse_marker)
}

fn parse_marker(line: &str) -> Option<SourcePosition> {
    let (line_no, rest) = line.split_once(',')?;
    let (col_no, _) = rest.split_once(':')?;
    if !is_number(line_no) || !is_number(col_no) {
        return None;
    }
    Some(SourcePosition {
        line: line_no.parse().ok()?,
        column: col_no.parse().ok()?,
    })
}

fn is_number(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_piston_meta_marker() {
        let message = "Expected something\n3,5: x = \"two\"\n3,5:     ^\n";
        assert_eq!(
            parse_position(message),
            Some(SourcePosition { line: 3, column: 5 })
        );
    }

    #[test]
    fn ignores_non_marker_lines() {
        let message = "Error ExpectedTag\n2: fn main() {\n\n41,12:     x := )\n";
        assert_eq!(
            parse_position(message),
            Some(SourcePosition {
                line: 41,
                column: 12
            })
        );
    }

    #[test]
    fn returns_none_without_marker() {
        assert_eq!(
            parse_position("Could not open `missing.dyon`, not found"),
            None
        );
        assert_eq!(parse_position("Could not find function `main`"), None);
    }
}
