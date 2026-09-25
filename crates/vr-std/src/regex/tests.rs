//! Unit tests for the regular-expression commands.

use super::*;

#[test]
fn regex_match_is_an_unanchored_search() {
    assert!(regex_match("Rust", "VisualRust").unwrap());
    assert!(regex_match("^a.c$", "abc").unwrap());
    assert!(!regex_match("^a.c$", "abcd").unwrap());
    assert!(regex_match(r"\d+", "abc123").unwrap());
    assert!(!regex_match(r"\d+", "abc").unwrap());
}

#[test]
fn regex_match_rejects_invalid_patterns() {
    let error = regex_match("(", "abc").unwrap_err();
    assert!(matches!(
        error,
        RegexError::InvalidPattern {
            function: "regex_match",
            ..
        }
    ));
}

#[test]
fn regex_replace_rewrites_every_match() {
    assert_eq!(regex_replace("a", "b", "banana").unwrap(), "bbnbnb");
    assert_eq!(regex_replace(r"\d+", "#", "a1b22c333").unwrap(), "a#b#c#");
    assert_eq!(regex_replace("z", "x", "abc").unwrap(), "abc");
}

#[test]
fn regex_replace_supports_capture_groups() {
    assert_eq!(
        regex_replace(r"(\w+)@(\w+)", "$2.$1", "user@host").unwrap(),
        "host.user"
    );
}

#[test]
fn regex_replace_rejects_invalid_patterns() {
    let error = regex_replace("[", "x", "abc").unwrap_err();
    assert!(matches!(
        error,
        RegexError::InvalidPattern {
            function: "regex_replace",
            ..
        }
    ));
}
