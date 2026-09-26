//! Escaping for Dyon string literals.
//!
//! Dyon's lexer reads strings with the JSON escape set (`\"`, `\\`, `\/`,
//! `\b`, `\f`, `\n`, `\r`, `\t`, `\uXXXX`), so a string escaped the JSON way is
//! always safe here. Non-ASCII is emitted verbatim instead of `\uXXXX` because
//! Dyon source is UTF-8: keeping `café` and `☃` readable matters more than an
//! all-ASCII file.

/// Escapes `value` so it can sit inside a Dyon double-quoted string literal.
pub fn escape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0c}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

/// Escapes `value` and wraps it in double quotes.
pub fn quoted(value: &str) -> String {
    format!("\"{}\"", escape(value))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_quotes_backslashes_and_newlines() {
        assert_eq!(escape(r#"say "hi""#), r#"say \"hi\""#);
        assert_eq!(escape(r"a\b"), r"a\\b");
        assert_eq!(escape("a\nb\tc\rd"), r"a\nb\tc\rd");
    }

    #[test]
    fn escapes_other_control_characters_as_unicode() {
        assert_eq!(escape("\u{01}\u{08}\u{0c}"), r"\u0001\b\f");
    }

    #[test]
    fn keeps_unicode_verbatim() {
        assert_eq!(escape("café ☃"), "café ☃");
        assert_eq!(quoted("café ☃"), "\"café ☃\"");
    }

    #[test]
    fn wraps_in_quotes() {
        assert_eq!(quoted("ok"), "\"ok\"");
    }
}
