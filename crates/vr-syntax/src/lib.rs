//! Dyon lexer and Scintilla style mapping.
//!
//! This crate is the pure-Rust half of the container-lexer highlighting path in
//! `docs/PLAN.md`: the editor answers `SCN_STYLENEEDED` by asking [`Lexer`] for
//! style runs and feeding them to `SCI_SETSTYLING`. No Win32 or Scintilla type
//! leaks in, so the lexer runs and is tested without a window session.
//!
//! The lexer is incremental. [`Lexer::style_all`] does the initial full pass and
//! caches one entry per source line; [`Lexer::restyle_lines`] resumes at a line
//! boundary using the cached state (for example "inside a block comment") and
//! re-scans only from there. Style runs are `(start_byte, len, StyleKind)` and
//! [`StyleKind::index`] gives the Scintilla style number.
//!
//! ```
//! use vr_syntax::{Lexer, StyleKind};
//!
//! let mut lexer = Lexer::new();
//! let runs = lexer.style_all("fn main() { return 0; }");
//! assert_eq!(runs[0].kind, StyleKind::Keyword);
//! assert_eq!(runs[0].start, 0);
//! ```

#![forbid(unsafe_code)]

mod lexeme;
mod lexer;
mod scan;
mod style;

pub use lexer::{Lexer, LineState};
pub use style::{STYLE_COUNT, StyleKind, StyleRun, TokenKind};
