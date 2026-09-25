//! The incremental Dyon lexer.
//!
//! [`Lexer::style_all`] performs the first, full pass and caches the state and
//! runs of every line. [`Lexer::restyle_lines`] then resumes at a line boundary
//! using the cached state of the line before it, so a keystroke near the end of
//! a large file re-scans only the tail — never the whole document.

use crate::scan;
use crate::style::{StyleRun, push_run};

/// The lexical state carried from one source line to the next.
///
/// Only block comments survive a newline; a string or comment that is not
/// closed by the end of its line ends there.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LineState {
    /// Ordinary Dyon text.
    #[default]
    Normal,
    /// Inside a `/* ... */` comment; `depth` counts open nesting and `doc`
    /// remembers whether it opened as a documentation comment.
    BlockComment {
        /// Number of `/*` still to be closed.
        depth: u32,
        /// Whether the comment was opened with `/**`.
        doc: bool,
    },
}

/// One cached source line.
struct Line {
    /// Byte offset where the line begins.
    start: usize,
    /// Byte offset one past the line's trailing newline (or the document end).
    end: usize,
    /// State entering the line.
    state_in: LineState,
    /// State leaving the line.
    state_out: LineState,
    /// Styles for the line, relative to `start`.
    runs: Vec<StyleRun>,
}

/// A reusable, incrementally-updated Dyon lexer.
pub struct Lexer {
    lines: Vec<Line>,
    last_scanned: usize,
}

impl Lexer {
    /// Creates an empty lexer; the first [`Lexer::style_all`] populates it.
    pub const fn new() -> Lexer {
        Lexer {
            lines: Vec::new(),
            last_scanned: 0,
        }
    }

    /// Lexes `text` from scratch, replaces the cache and returns every run with
    /// document-relative offsets.
    pub fn style_all(&mut self, text: &str) -> Vec<StyleRun> {
        self.lines.clear();
        let mut state = LineState::Normal;
        let mut scanned = 0;
        for (start, end, line) in split_lines(text, 0) {
            let (runs, state_out) = scan::scan_line(line, state);
            self.lines.push(Line {
                start,
                end,
                state_in: state,
                state_out,
                runs,
            });
            state = state_out;
            scanned += 1;
        }
        self.last_scanned = scanned;
        self.absolute_runs(0)
    }

    /// Re-lexes from the start of `start_line` to the document end, reusing the
    /// cached state entering that line, and returns the runs for the re-lexed
    /// region. Lines before `start_line` are left untouched.
    ///
    /// `text` is the whole current document; the caller passes the first line an
    /// edit could affect. A `start_line` past the cached extent is clamped.
    pub fn restyle_lines(&mut self, text: &str, start_line: usize) -> Vec<StyleRun> {
        let start_line = start_line.min(self.lines.len());
        let (start_offset, state) = if start_line == 0 {
            (0, LineState::Normal)
        } else {
            match self.lines.get(start_line - 1) {
                Some(previous) => (previous.end, previous.state_out),
                None => (0, LineState::Normal),
            }
        };
        self.lines.truncate(start_line);
        self.lex_from(text, start_offset, state);
        self.absolute_runs(start_line)
    }

    /// Incrementally restyles the byte range `[from, to)`, returning runs clipped
    /// to that range. Re-lexing still starts at the beginning of the line that
    /// contains `from`, because a style cannot be computed without the state
    /// entering that line.
    pub fn restyle_range(&mut self, text: &str, from: usize, to: usize) -> Vec<StyleRun> {
        let line = self.line_at_byte(from);
        let runs = self.restyle_lines(text, line);
        let from = from.min(text.len());
        let to = to.max(from).min(text.len());
        clip_runs(&runs, from, to)
    }

    /// The number of lines currently cached.
    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    /// The state entering `line`, from the cache.
    pub fn state_at_line(&self, line: usize) -> Option<LineState> {
        self.lines.get(line).map(|cached| cached.state_in)
    }

    /// The state leaving `line`, from the cache.
    pub fn end_state(&self, line: usize) -> Option<LineState> {
        self.lines.get(line).map(|cached| cached.state_out)
    }

    /// How many lines the most recent call scanned. This is the observable
    /// proof that a restyle does not rescan the prefix.
    pub fn last_scanned_lines(&self) -> usize {
        self.last_scanned
    }

    fn lex_from(&mut self, text: &str, from: usize, mut state: LineState) {
        let from = floor_char_boundary(text, from.min(text.len()));
        let mut scanned = 0;
        for (start, end, line) in split_lines(text, from) {
            let (runs, state_out) = scan::scan_line(line, state);
            self.lines.push(Line {
                start,
                end,
                state_in: state,
                state_out,
                runs,
            });
            state = state_out;
            scanned += 1;
        }
        self.last_scanned = scanned;
    }

    /// Concatenates the cached runs from `from_line` onwards at their absolute
    /// offsets, coalescing styles that meet across a line break.
    fn absolute_runs(&self, from_line: usize) -> Vec<StyleRun> {
        let Some(lines) = self.lines.get(from_line..) else {
            return Vec::new();
        };
        let mut runs = Vec::new();
        for line in lines {
            for run in &line.runs {
                push_run(&mut runs, line.start + run.start, run.len, run.kind);
            }
        }
        runs
    }

    /// The index of the cached line containing `byte`. A `byte` beyond the tail
    /// falls back to the last cached line, which is still a sound restart point.
    fn line_at_byte(&self, byte: usize) -> usize {
        if self.lines.is_empty() {
            return 0;
        }
        let index = self.lines.partition_point(|line| line.end <= byte);
        index.min(self.lines.len() - 1)
    }
}

impl Default for Lexer {
    fn default() -> Lexer {
        Lexer::new()
    }
}

/// Splits `text[from..]` into `(start, end, line)` triples. The newline stays on
/// the line it terminates so byte offsets tile the document without gaps.
fn split_lines(text: &str, from: usize) -> impl Iterator<Item = (usize, usize, &str)> {
    let mut start = from.min(text.len());
    std::iter::from_fn(move || {
        if start >= text.len() {
            return None;
        }
        let tail = &text[start..];
        let end = match tail.find('\n') {
            Some(offset) => start + offset + 1,
            None => text.len(),
        };
        let line = &text[start..end];
        let line_start = start;
        start = end;
        Some((line_start, end, line))
    })
}

/// Intersects each run with `[from, to)`, dropping anything outside and merging
/// fragments that become adjacent.
fn clip_runs(runs: &[StyleRun], from: usize, to: usize) -> Vec<StyleRun> {
    let mut out = Vec::new();
    for run in runs {
        let start = run.start.max(from);
        let end = (run.start + run.len).min(to);
        if start < end {
            push_run(&mut out, start, end - start, run.kind);
        }
    }
    out
}

/// Clamps `index` down to the nearest UTF-8 character boundary.
fn floor_char_boundary(text: &str, index: usize) -> usize {
    let mut index = index;
    while index > 0 && !text.is_char_boundary(index) {
        index -= 1;
    }
    index
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::StyleKind;

    fn spans(runs: &[StyleRun]) -> Vec<(usize, usize, StyleKind)> {
        runs.iter()
            .map(|run| (run.start, run.len, run.kind))
            .collect()
    }

    #[test]
    fn style_all_caches_every_line_and_offsets_runs() {
        let mut lexer = Lexer::new();
        let text = "fn a() {}\nmut b = 1;\n";
        let runs = lexer.style_all(text);
        assert_eq!(lexer.line_count(), 2);
        assert_eq!(lexer.state_at_line(0), Some(LineState::Normal));
        // "fn a() {}\n" is 10 bytes, so line 1 starts at 10.
        assert_eq!(
            spans(&runs),
            vec![
                (0, 2, StyleKind::Keyword),
                (2, 1, StyleKind::Default),
                (3, 1, StyleKind::Function),
                (4, 2, StyleKind::Operator),
                (6, 1, StyleKind::Default),
                (7, 2, StyleKind::Operator),
                (9, 1, StyleKind::Default),
                (10, 3, StyleKind::Keyword),
                (13, 3, StyleKind::Default),
                (16, 1, StyleKind::Operator),
                (17, 1, StyleKind::Default),
                (18, 1, StyleKind::Number),
                (19, 1, StyleKind::Operator),
                (20, 1, StyleKind::Default),
            ]
        );
    }

    #[test]
    fn block_comment_state_flows_across_lines() {
        let mut lexer = Lexer::new();
        let text = "fn a() { /* start\nstill */ return 1; }\n";
        lexer.style_all(text);
        assert_eq!(
            lexer.end_state(0),
            Some(LineState::BlockComment {
                depth: 1,
                doc: false
            })
        );
        assert_eq!(lexer.state_at_line(1), lexer.end_state(0));
        // Line 1 begins inside the comment, so its first run is a comment.
        // "fn a() { /* start\n" is 18 bytes, so line 1 starts at 18.
        let runs = lexer.restyle_lines(text, 1);
        assert_eq!(
            spans(&runs),
            vec![
                (18, 8, StyleKind::Comment),
                (26, 1, StyleKind::Default),
                (27, 6, StyleKind::Keyword),
                (33, 1, StyleKind::Default),
                (34, 1, StyleKind::Number),
                (35, 1, StyleKind::Operator),
                (36, 1, StyleKind::Default),
                (37, 1, StyleKind::Operator),
                (38, 1, StyleKind::Default),
            ]
        );
    }

    #[test]
    fn edit_rescans_only_from_the_changed_line() {
        let mut lexer = Lexer::new();
        let original = "fn a() { /* c\n line */ return 1; }\n";
        lexer.style_all(original);
        assert_eq!(lexer.line_count(), 2);

        let edited = "fn a() { /* c\n line */ return 2; }\n";
        lexer.restyle_lines(edited, 1);
        assert_eq!(lexer.last_scanned_lines(), 1, "only line 1 is re-lexed");
        assert_eq!(
            lexer.state_at_line(1),
            Some(LineState::BlockComment {
                depth: 1,
                doc: false
            }),
            "the cached comment state entering line 1 is reused"
        );

        lexer.restyle_lines(edited, 0);
        assert_eq!(lexer.last_scanned_lines(), 2);
    }

    #[test]
    fn style_all_after_restyle_rebuilds_the_cache() {
        let mut lexer = Lexer::new();
        lexer.style_all("a\nb\nc\n");
        assert_eq!(lexer.line_count(), 3);
        lexer.style_all("only\n");
        assert_eq!(lexer.line_count(), 1);
    }

    #[test]
    fn restyle_range_clips_to_the_requested_bytes() {
        let mut lexer = Lexer::new();
        let runs = lexer.restyle_range("return 1;\n", 0, 8);
        assert_eq!(
            spans(&runs),
            vec![
                (0, 6, StyleKind::Keyword),
                (6, 1, StyleKind::Default),
                (7, 1, StyleKind::Number),
            ]
        );
    }

    #[test]
    fn restyle_range_resumes_inside_a_line() {
        let mut lexer = Lexer::new();
        lexer.style_all("fn a() { /* c\n line */ return 1; }\n");
        let runs = lexer.restyle_range("fn a() { /* c\n line */ return 2; }\n", 23, 31);
        assert_eq!(
            spans(&runs),
            vec![
                (23, 6, StyleKind::Keyword),
                (29, 1, StyleKind::Default),
                (30, 1, StyleKind::Number),
            ]
        );
        assert_eq!(lexer.last_scanned_lines(), 1);
    }

    #[test]
    fn restyle_past_the_cached_tail_is_safe() {
        let mut lexer = Lexer::new();
        lexer.style_all("a\n");
        let runs = lexer.restyle_lines("a\nb\n", 2);
        assert_eq!(spans(&runs), vec![(2, 2, StyleKind::Default)]);
        assert_eq!(lexer.line_count(), 2);
    }
}
