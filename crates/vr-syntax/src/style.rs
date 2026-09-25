//! Lexical token categories and their Scintilla style mapping.
//!
//! The scanner classifies bytes into [`TokenKind`]s; the editor only ever sees
//! [`StyleKind`]s, each of which has a fixed Scintilla style number. Keeping the
//! two taxonomies separate means the token set can grow (a new keyword style,
//! say) without renumbering the styles Scintilla already knows about.

/// A byte range carrying one lexical category.
///
/// `start` is relative to whatever origin the producer documented: line-relative
/// for the per-line scanner, document-relative for [`crate::Lexer`] results.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StyleRun {
    /// Byte offset of the first byte in the run.
    pub start: usize,
    /// Byte length of the run; never zero.
    pub len: usize,
    /// The category the run is styled as.
    pub kind: StyleKind,
}

/// The lexical category a scanner token belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TokenKind {
    /// Spaces, tabs and line breaks.
    Whitespace,
    /// A reserved Dyon word.
    Keyword,
    /// A non-reserved identifier.
    Ident,
    /// The name declared after `fn`.
    Declaration,
    /// A `//` or `/* */` comment.
    Comment,
    /// A `///`, `//!` or `/** */` documentation comment.
    DocComment,
    /// A double-quoted string literal.
    StringLiteral,
    /// An integer, float or exponent literal.
    Number,
    /// Operator or punctuation.
    Operator,
    /// The `link` keyword introducing a native link block.
    Link,
    /// A byte that cannot start a Dyon token.
    Error,
}

impl TokenKind {
    /// The style the editor paints this token with.
    pub const fn style(self) -> StyleKind {
        match self {
            TokenKind::Whitespace => StyleKind::Default,
            TokenKind::Keyword => StyleKind::Keyword,
            TokenKind::Ident => StyleKind::Default,
            TokenKind::Declaration => StyleKind::Function,
            TokenKind::Comment => StyleKind::Comment,
            TokenKind::DocComment => StyleKind::DocComment,
            TokenKind::StringLiteral => StyleKind::String,
            TokenKind::Number => StyleKind::Number,
            TokenKind::Operator => StyleKind::Operator,
            TokenKind::Link => StyleKind::Link,
            TokenKind::Error => StyleKind::Error,
        }
    }
}

/// A visual category, numbered for Scintilla's `SCI_SETSTYLING`.
///
/// The discriminants are the Scintilla style indices. They stay small because
/// `SCI_STARTSTYLING` only carries five style bits.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StyleKind {
    /// Plain text and whitespace.
    Default = 0,
    /// Reserved words.
    Keyword = 1,
    /// Names declared with `fn`.
    Function = 2,
    /// `//` and `/* */` comments.
    Comment = 3,
    /// Documentation comments.
    DocComment = 4,
    /// String literals.
    String = 5,
    /// Numeric literals.
    Number = 6,
    /// Operators and punctuation.
    Operator = 7,
    /// The `link` keyword and native link declarations.
    Link = 8,
    /// Text that cannot be lexed.
    Error = 9,
}

/// Number of Scintilla styles this crate defines.
pub const STYLE_COUNT: u8 = 10;

impl StyleKind {
    /// The Scintilla style index for this category.
    pub const fn index(self) -> u8 {
        self as u8
    }

    /// Every style, in index order; the editor uses this to reset its palette.
    pub const ALL: [StyleKind; STYLE_COUNT as usize] = [
        StyleKind::Default,
        StyleKind::Keyword,
        StyleKind::Function,
        StyleKind::Comment,
        StyleKind::DocComment,
        StyleKind::String,
        StyleKind::Number,
        StyleKind::Operator,
        StyleKind::Link,
        StyleKind::Error,
    ];
}

/// Appends a run, merging it into the previous one when both are the same style
/// and touch. Coalescing here keeps runs minimal without a second pass.
pub(crate) fn push_run(runs: &mut Vec<StyleRun>, start: usize, len: usize, kind: StyleKind) {
    if len == 0 {
        return;
    }
    if let Some(last) = runs.last_mut()
        && last.kind == kind
        && last.start + last.len == start
    {
        last.len += len;
        return;
    }
    runs.push(StyleRun { start, len, kind });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn style_indices_are_the_palette_order() {
        for (index, style) in StyleKind::ALL.into_iter().enumerate() {
            assert_eq!(usize::from(style.index()), index);
        }
        assert_eq!(StyleKind::Error.index(), STYLE_COUNT - 1);
    }

    #[test]
    fn tokens_map_to_their_style() {
        assert_eq!(TokenKind::Whitespace.style(), StyleKind::Default);
        assert_eq!(TokenKind::Declaration.style(), StyleKind::Function);
        assert_eq!(TokenKind::Link.style(), StyleKind::Link);
        assert_eq!(TokenKind::Error.style(), StyleKind::Error);
    }

    #[test]
    fn push_run_merges_touching_equal_runs() {
        let mut runs = Vec::new();
        push_run(&mut runs, 0, 2, StyleKind::Keyword);
        push_run(&mut runs, 2, 3, StyleKind::Keyword);
        push_run(&mut runs, 5, 1, StyleKind::Default);
        push_run(&mut runs, 7, 1, StyleKind::Keyword);
        push_run(&mut runs, 0, 0, StyleKind::Error);
        assert_eq!(
            runs,
            vec![
                StyleRun {
                    start: 0,
                    len: 5,
                    kind: StyleKind::Keyword
                },
                StyleRun {
                    start: 5,
                    len: 1,
                    kind: StyleKind::Default
                },
                StyleRun {
                    start: 7,
                    len: 1,
                    kind: StyleKind::Keyword
                },
            ]
        );
    }
}
