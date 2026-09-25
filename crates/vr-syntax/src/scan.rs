//! The per-line Dyon scanner.
//!
//! A line is the unit of incremental work: the scanner is handed one line and
//! the lexer state left by the previous line, and returns runs relative to the
//! line plus the state to hand to the next line. Only block comments carry state
//! across lines, because every other Dyon token is terminated by a line break.

use crate::lexeme::{
    is_doc_line_comment, is_ident_continue, is_ident_start, scan_number, scan_operator, scan_string,
};
use crate::lexer::LineState;
use crate::style::{StyleKind, StyleRun, TokenKind, push_run};

/// Reserved words. `link` is handled separately because it gets its own style.
const KEYWORDS: &[&str] = &[
    "fn", "for", "loop", "if", "else", "return", "break", "continue", "in", "go", "ns", "mut",
    "use", "import", "true", "false", "none", "some", "ok", "err",
];

/// Scans one line, resuming from `state_in`, and returns line-relative style
/// runs plus the state entering the next line.
pub(crate) fn scan_line(line: &str, state_in: LineState) -> (Vec<StyleRun>, LineState) {
    let bytes = line.as_bytes();
    let mut runs = Vec::new();
    let mut state = state_in;
    let mut index = 0usize;
    // Set after a `fn` keyword until the declared name is consumed.
    let mut expect_declaration = false;

    if let LineState::BlockComment { depth, doc } = state {
        let mut depth = depth;
        let (end, next) = scan_block_comment(bytes, index, &mut depth, doc);
        let token = if doc {
            TokenKind::DocComment
        } else {
            TokenKind::Comment
        };
        push_run(&mut runs, 0, end, token.style());
        index = end;
        state = next;
    }

    while index < bytes.len() {
        let byte = bytes[index];
        match byte {
            b'/' if bytes.get(index + 1) == Some(&b'/') => {
                let token = if is_doc_line_comment(bytes, index) {
                    TokenKind::DocComment
                } else {
                    TokenKind::Comment
                };
                push_run(&mut runs, index, bytes.len() - index, token.style());
                index = bytes.len();
            }
            b'/' if bytes.get(index + 1) == Some(&b'*') => {
                let doc =
                    bytes.get(index + 2) == Some(&b'*') && bytes.get(index + 3) != Some(&b'/');
                let start = index;
                let mut depth = 1u32;
                let (end, next) = scan_block_comment(bytes, index + 2, &mut depth, doc);
                let token = if doc {
                    TokenKind::DocComment
                } else {
                    TokenKind::Comment
                };
                push_run(&mut runs, start, end - start, token.style());
                index = end;
                state = next;
            }
            b'"' => {
                let end = scan_string(bytes, index);
                push_run(&mut runs, index, end - index, StyleKind::String);
                index = end;
            }
            _ if byte.is_ascii_whitespace() => {
                let start = index;
                while index < bytes.len() && bytes[index].is_ascii_whitespace() {
                    index += 1;
                }
                push_run(&mut runs, start, index - start, StyleKind::Default);
            }
            _ if byte.is_ascii_digit() => {
                let end = scan_number(bytes, index);
                push_run(&mut runs, index, end - index, StyleKind::Number);
                index = end;
                expect_declaration = false;
            }
            _ if is_ident_start(byte) => {
                let start = index;
                while index < bytes.len() && is_ident_continue(bytes[index]) {
                    index += 1;
                }
                let word = &line[start..index];
                let token = if expect_declaration {
                    TokenKind::Declaration
                } else if word == "link" {
                    TokenKind::Link
                } else if KEYWORDS.contains(&word) {
                    TokenKind::Keyword
                } else {
                    TokenKind::Ident
                };
                expect_declaration = word == "fn";
                push_run(&mut runs, start, index - start, token.style());
            }
            _ => {
                let (length, token) = scan_operator(bytes, index);
                push_run(&mut runs, index, length, token.style());
                index += length;
                expect_declaration = false;
            }
        }
    }

    (runs, state)
}

/// Consumes a block comment body starting just after an opening `/*`, tracking
/// nesting so `/* /* */ */` closes once. Returns the exclusive end and the state
/// left behind. `doc` remembers whether the comment opened with `/**` so a
/// multi-line documentation comment keeps its style.
fn scan_block_comment(
    bytes: &[u8],
    mut index: usize,
    depth: &mut u32,
    doc: bool,
) -> (usize, LineState) {
    while index < bytes.len() {
        match (bytes[index], bytes.get(index + 1)) {
            (b'/', Some(&b'*')) => {
                *depth += 1;
                index += 2;
            }
            (b'*', Some(&b'/')) => {
                *depth -= 1;
                index += 2;
                if *depth == 0 {
                    return (index, LineState::Normal);
                }
            }
            _ => index += 1,
        }
    }
    // Unterminated: the comment continues on the next line at the same depth.
    (bytes.len(), LineState::BlockComment { depth: *depth, doc })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spans(runs: &[StyleRun]) -> Vec<(usize, usize, StyleKind)> {
        runs.iter()
            .map(|run| (run.start, run.len, run.kind))
            .collect()
    }

    #[test]
    fn keyword_function_and_operators() {
        let (runs, state) = scan_line("fn main() { return 0; }", LineState::Normal);
        assert_eq!(state, LineState::Normal);
        assert_eq!(
            spans(&runs),
            vec![
                (0, 2, StyleKind::Keyword),
                (2, 1, StyleKind::Default),
                (3, 4, StyleKind::Function),
                (7, 2, StyleKind::Operator),
                (9, 1, StyleKind::Default),
                (10, 1, StyleKind::Operator),
                (11, 1, StyleKind::Default),
                (12, 6, StyleKind::Keyword),
                (18, 1, StyleKind::Default),
                (19, 1, StyleKind::Number),
                (20, 1, StyleKind::Operator),
                (21, 1, StyleKind::Default),
                (22, 1, StyleKind::Operator),
            ]
        );
    }

    #[test]
    fn line_and_doc_comments() {
        let (line, _) = scan_line("// hi\n", LineState::Normal);
        assert_eq!(spans(&line), vec![(0, 6, StyleKind::Comment)]);
        let (doc, _) = scan_line("/// doc\n", LineState::Normal);
        assert_eq!(spans(&doc), vec![(0, 8, StyleKind::DocComment)]);
        let (bang, _) = scan_line("//! doc\n", LineState::Normal);
        assert_eq!(spans(&bang), vec![(0, 8, StyleKind::DocComment)]);
        // Four slashes are an ordinary comment, not a doc comment.
        let (plain, _) = scan_line("//// plain\n", LineState::Normal);
        assert_eq!(spans(&plain), vec![(0, 11, StyleKind::Comment)]);
    }

    #[test]
    fn block_comment_opens_and_continues() {
        let (runs, state) = scan_line("/* start\n", LineState::Normal);
        assert_eq!(
            state,
            LineState::BlockComment {
                depth: 1,
                doc: false
            }
        );
        assert_eq!(spans(&runs), vec![(0, 9, StyleKind::Comment)]);

        let (runs, state) = scan_line(
            "still */ link { fn ext() }\n",
            LineState::BlockComment {
                depth: 1,
                doc: false,
            },
        );
        assert_eq!(state, LineState::Normal);
        assert_eq!(
            spans(&runs),
            vec![
                (0, 8, StyleKind::Comment),
                (8, 1, StyleKind::Default),
                (9, 4, StyleKind::Link),
                (13, 1, StyleKind::Default),
                (14, 1, StyleKind::Operator),
                (15, 1, StyleKind::Default),
                (16, 2, StyleKind::Keyword),
                (18, 1, StyleKind::Default),
                (19, 3, StyleKind::Function),
                (22, 2, StyleKind::Operator),
                (24, 1, StyleKind::Default),
                (25, 1, StyleKind::Operator),
                (26, 1, StyleKind::Default),
            ]
        );
    }

    #[test]
    fn doc_block_comment_keeps_its_kind_across_lines() {
        let (runs, state) = scan_line("/** doc\n", LineState::Normal);
        assert_eq!(
            state,
            LineState::BlockComment {
                depth: 1,
                doc: true
            }
        );
        assert_eq!(spans(&runs), vec![(0, 8, StyleKind::DocComment)]);

        let (runs, state) = scan_line(
            "more */ x\n",
            LineState::BlockComment {
                depth: 1,
                doc: true,
            },
        );
        assert_eq!(state, LineState::Normal);
        assert_eq!(
            spans(&runs),
            vec![(0, 7, StyleKind::DocComment), (7, 3, StyleKind::Default)]
        );
    }

    #[test]
    fn nested_block_comment_closes_once() {
        let (runs, state) = scan_line("/* a /* b */ c */\n", LineState::Normal);
        assert_eq!(state, LineState::Normal);
        assert_eq!(
            spans(&runs),
            vec![(0, 17, StyleKind::Comment), (17, 1, StyleKind::Default)]
        );
    }

    #[test]
    fn strings_honour_escapes() {
        let (runs, _) = scan_line("mut x = \"a\\\"b\\n\"", LineState::Normal);
        assert_eq!(
            spans(&runs),
            vec![
                (0, 3, StyleKind::Keyword),
                (3, 3, StyleKind::Default),
                (6, 1, StyleKind::Operator),
                (7, 1, StyleKind::Default),
                (8, 8, StyleKind::String),
            ]
        );
    }

    #[test]
    fn numbers_cover_floats_exponents_and_suffixes() {
        let (runs, _) = scan_line("1e3 1.5 2.5e-10 0xff 3u32", LineState::Normal);
        assert_eq!(
            spans(&runs),
            vec![
                (0, 3, StyleKind::Number),
                (3, 1, StyleKind::Default),
                (4, 3, StyleKind::Number),
                (7, 1, StyleKind::Default),
                (8, 7, StyleKind::Number),
                (15, 1, StyleKind::Default),
                (16, 4, StyleKind::Number),
                (20, 1, StyleKind::Default),
                (21, 4, StyleKind::Number),
            ]
        );
    }

    #[test]
    fn link_in_go_and_question_mark() {
        let (link, _) = scan_line("link", LineState::Normal);
        assert_eq!(spans(&link), vec![(0, 4, StyleKind::Link)]);
        let (block, _) = scan_line("link {}", LineState::Normal);
        assert_eq!(
            spans(&block),
            vec![
                (0, 4, StyleKind::Link),
                (4, 1, StyleKind::Default),
                (5, 2, StyleKind::Operator),
            ]
        );
        let (go, _) = scan_line("go", LineState::Normal);
        assert_eq!(spans(&go), vec![(0, 2, StyleKind::Keyword)]);
        let (in_, _) = scan_line("in", LineState::Normal);
        assert_eq!(spans(&in_), vec![(0, 2, StyleKind::Keyword)]);
        let (question, _) = scan_line("f()?", LineState::Normal);
        assert_eq!(
            spans(&question),
            vec![(0, 1, StyleKind::Default), (1, 3, StyleKind::Operator)]
        );
    }

    #[test]
    fn invalid_bytes_become_errors() {
        // A backtick (invalid punctuation) and a multi-byte char merge into one
        // error run through the same coalescing path as every other style.
        let (runs, _) = scan_line("`\u{00e9}", LineState::Normal);
        assert_eq!(spans(&runs), vec![(0, 3, StyleKind::Error)]);
    }
}
