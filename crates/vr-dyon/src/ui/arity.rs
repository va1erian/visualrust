//! Handler parameter counts, scanned from the Dyon source once.
//!
//! Dyon does not expose a loaded function's signature through [`dyon::Module`],
//! but the dispatch step needs to know whether to pass the typed event payload:
//! a generated handler is zero-argument and must receive none, while a
//! hand-written handler that declares a parameter expects the payload. The
//! source is scanned before it is loaded and the counts are kept per thread.

use std::cell::RefCell;
use std::collections::HashMap;

thread_local! {
    static ARITIES: RefCell<HashMap<String, usize>> = RefCell::new(HashMap::new());
}

/// Records the handler arities for the program about to run.
pub(crate) fn set_arities(source: &str) {
    ARITIES.with(|arities| *arities.borrow_mut() = scan_arities(source));
}

/// The recorded arities, cloned for the frozen plan.
pub(crate) fn arities() -> HashMap<String, usize> {
    ARITIES.with(|arities| arities.borrow().clone())
}

/// Scans `fn name(` declarations and counts their parameters. Comments and
/// string literals are stripped first so a `fn` inside text is not counted.
fn scan_arities(source: &str) -> HashMap<String, usize> {
    let cleaned = strip_literals(source);
    let mut arities = HashMap::new();
    let bytes = cleaned.as_bytes();
    let mut index = 0;
    while index + 3 < bytes.len() {
        if &cleaned[index..index + 2] == "fn"
            && boundary(bytes, index)
            && let Some((name, arity, next)) = parse_signature(&cleaned, index + 2)
        {
            arities.insert(name, arity);
            index = next;
            continue;
        }
        index += 1;
    }
    arities
}

/// Whether `index` is a word boundary on both sides of a two-letter keyword.
fn boundary(bytes: &[u8], index: usize) -> bool {
    let before = index == 0 || !is_ident(bytes[index - 1]);
    let after = index + 2 >= bytes.len() || !is_ident(bytes[index + 2]);
    before && after
}

fn is_ident(byte: u8) -> bool {
    byte == b'_' || byte.is_ascii_alphanumeric()
}

/// Parses ` name(a, b)`, returning the name, arity and the index after `)`.
fn parse_signature(source: &str, mut index: usize) -> Option<(String, usize, usize)> {
    let bytes = source.as_bytes();
    while index < bytes.len() && bytes[index].is_ascii_whitespace() {
        index += 1;
    }
    let start = index;
    while index < bytes.len() && is_ident(bytes[index]) {
        index += 1;
    }
    if start == index {
        return None;
    }
    let name = source[start..index].to_owned();
    while index < bytes.len() && bytes[index].is_ascii_whitespace() {
        index += 1;
    }
    if index >= bytes.len() || bytes[index] != b'(' {
        return None;
    }
    index += 1;
    let mut depth = 1;
    let start = index;
    while index < bytes.len() && depth > 0 {
        match bytes[index] {
            b'(' => depth += 1,
            b')' => depth -= 1,
            _ => {}
        }
        if depth > 0 {
            index += 1;
        }
    }
    if depth != 0 {
        return None;
    }
    let params = source[start..index].trim();
    let arity = if params.is_empty() {
        0
    } else {
        params.split(',').count()
    };
    Some((name, arity, index + 1))
}

/// Replaces comment and string contents with spaces so keyword scanning sees
/// only code, keeping byte positions stable.
fn strip_literals(source: &str) -> String {
    let mut out = String::with_capacity(source.len());
    let bytes = source.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'/' if index + 1 < bytes.len() && bytes[index + 1] == b'/' => {
                while index < bytes.len() && bytes[index] != b'\n' {
                    out.push(' ');
                    index += 1;
                }
            }
            b'/' if index + 1 < bytes.len() && bytes[index + 1] == b'*' => {
                out.push(' ');
                out.push(' ');
                index += 2;
                while index + 1 < bytes.len() && !(bytes[index] == b'*' && bytes[index + 1] == b'/')
                {
                    out.push(' ');
                    index += 1;
                }
                if index + 1 < bytes.len() {
                    out.push(' ');
                    out.push(' ');
                    index += 2;
                }
            }
            b'"' => {
                out.push(' ');
                index += 1;
                while index < bytes.len() && bytes[index] != b'"' {
                    if bytes[index] == b'\\' && index + 1 < bytes.len() {
                        out.push(' ');
                        index += 1;
                    }
                    out.push(' ');
                    index += 1;
                }
                if index < bytes.len() {
                    out.push(' ');
                    index += 1;
                }
            }
            other => {
                out.push(other as char);
                index += 1;
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scan_counts_parameters_and_ignores_literals() {
        let source = r#"
            // fn commented(a, b, c) {}
            fn on_click() {}
            fn on_change(text) {}
            fn on_select(index, extra) {}
            /* fn block(x, y) {} */
            fn with_string() { x := "fn fake()" }
        "#;
        let arities = scan_arities(source);
        assert_eq!(arities.get("on_click"), Some(&0));
        assert_eq!(arities.get("on_change"), Some(&1));
        assert_eq!(arities.get("on_select"), Some(&2));
        assert_eq!(arities.get("with_string"), Some(&0));
        assert!(!arities.contains_key("commented"));
        assert!(!arities.contains_key("block"));
        assert!(!arities.contains_key("fake"));
    }
}
