//! Idempotent Dyon handler stub generation.
//!
//! Stubs match the #86 dispatch convention: a handler is a Dyon function that
//! takes the request object and returns `{status, body, content_type}`. Kept in
//! its own module so the text shaping is testable without a runtime.

use std::collections::BTreeSet;
use std::fmt::Write as _;

use crate::model::Endpoint;

/// The Dyon function stub for one endpoint, using its sample response.
pub fn handler_stub(endpoint: &Endpoint) -> String {
    let response = endpoint.response();
    let body = response.body.unwrap_or_default();
    let mut out = String::new();
    // `write!` to a `String` cannot fail; the ignored result keeps the crate
    // free of `unwrap` while preserving the infallible signature.
    let _ = writeln!(out, "fn {}(req: {{}}) -> {{}} {{", endpoint.handler());
    let _ = writeln!(
        out,
        "    return {{status: {}, body: \"{}\", content_type: \"{}\"}}",
        response.status,
        escape_dyon(&body),
        escape_dyon(&response.content_type),
    );
    out.push_str("}\n");
    out
}

/// Appends a stub for every endpoint whose handler is not already defined.
///
/// Running this over its own output appends nothing, which is the idempotency
/// the API editor relies on when re-scaffolding a growing project. A handler
/// that already exists is left byte-for-byte alone so hand-written bodies are
/// never clobbered.
pub fn generate_stubs(source: &str, endpoints: &[Endpoint]) -> String {
    let mut present = defined_handlers(source);
    let mut additions = String::new();
    for endpoint in endpoints {
        if present.contains(endpoint.handler()) {
            continue;
        }
        present.insert(endpoint.handler().to_owned());
        if !additions.is_empty() {
            additions.push('\n');
        }
        additions.push_str(&handler_stub(endpoint));
    }

    if additions.is_empty() {
        return source.to_owned();
    }

    let mut out = source.to_owned();
    if !out.is_empty() {
        if !out.ends_with('\n') {
            out.push('\n');
        }
        out.push('\n');
    }
    out.push_str(&additions);
    out
}

/// The Dyon function names already defined in `source`.
pub fn defined_handlers(source: &str) -> BTreeSet<String> {
    let code = strip_noise(source);
    let mut names = BTreeSet::new();
    for (index, _) in code.match_indices("fn") {
        if code[..index]
            .chars()
            .next_back()
            .is_some_and(is_identifier_char)
        {
            continue;
        }
        let after = code[index + 2..].trim_start();
        let name: String = after
            .chars()
            .take_while(|c| is_identifier_char(*c))
            .collect();
        if name.is_empty() {
            continue;
        }
        let rest = after[name.len()..].trim_start();
        if rest.starts_with('(') {
            names.insert(name);
        }
    }
    names
}

/// Replaces comments and string literals with spaces so `fn` inside them is not
/// mistaken for a definition. Dyon's own lexer is not reused because the
/// generator must stay usable while the editor holds a half-typed buffer.
fn strip_noise(source: &str) -> String {
    let mut out = String::with_capacity(source.len());
    let mut chars = source.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '/' if chars.peek() == Some(&'/') => {
                for next in chars.by_ref() {
                    if next == '\n' {
                        out.push('\n');
                        break;
                    }
                }
            }
            '/' if chars.peek() == Some(&'*') => {
                chars.next();
                let mut previous = '\0';
                for next in chars.by_ref() {
                    if previous == '*' && next == '/' {
                        break;
                    }
                    previous = next;
                }
                out.push(' ');
            }
            '"' => {
                out.push(' ');
                let mut escaped = false;
                for next in chars.by_ref() {
                    if escaped {
                        escaped = false;
                    } else if next == '\\' {
                        escaped = true;
                    } else if next == '"' {
                        break;
                    }
                }
            }
            _ => out.push(c),
        }
    }
    out
}

fn is_identifier_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// Escapes a Rust string for a Dyon double-quoted literal.
fn escape_dyon(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{defined_handlers, generate_stubs, handler_stub};
    use crate::model::Endpoint;
    use vr_core::manifest::{HttpMethod, SampleResponse};

    fn endpoints() -> Vec<Endpoint> {
        let mut show = Endpoint::new("show", HttpMethod::Get, "/todos/:id", "show_todo");
        show.set_response(SampleResponse {
            status: 200,
            body: Some("{\"id\":1}".to_owned()),
            content_type: "application/json".to_owned(),
        });
        vec![Endpoint::new("index", HttpMethod::Get, "/", "index"), show]
    }

    #[test]
    fn stub_matches_the_dispatch_shape() {
        let stub = handler_stub(&endpoints()[0]);
        assert_eq!(
            stub,
            "fn index(req: {}) -> {} {\n    return {status: 200, body: \"\", content_type: \"text/plain; charset=utf-8\"}\n}\n"
        );
    }

    #[test]
    fn escaping_keeps_the_literal_valid() {
        let mut endpoint = Endpoint::new("e", HttpMethod::Post, "/e", "escaped");
        endpoint.set_response(SampleResponse {
            status: 201,
            body: Some("say \"hi\"\nline".to_owned()),
            content_type: "text/plain".to_owned(),
        });
        let stub = handler_stub(&endpoint);
        assert!(stub.contains("body: \"say \\\"hi\\\"\\nline\""), "{stub}");
    }

    #[test]
    fn generation_twice_is_idempotent() {
        let endpoints = endpoints();
        let once = generate_stubs("// existing\n", &endpoints);
        let twice = generate_stubs(&once, &endpoints);

        assert_eq!(once, twice, "second run must not change the source");
        assert_eq!(once.matches("fn index(").count(), 1);
        assert_eq!(once.matches("fn show_todo(").count(), 1);
    }

    #[test]
    fn existing_handlers_are_not_duplicated() {
        let source = "fn index(req: {}) -> {} {\n    return {status: 204}\n}\n";
        let result = generate_stubs(source, &endpoints());
        assert_eq!(result.matches("fn index(").count(), 1);
        assert!(result.contains("status: 204"), "hand-written body survives");
        assert!(result.contains("fn show_todo("), "missing stub is added");
    }

    #[test]
    fn detection_ignores_comments_and_strings() {
        let source = "// fn commented()\nfn real() -> {} {\n    return \"fn in string\"\n}\n";
        let names = defined_handlers(source);
        assert!(names.contains("real"));
        assert!(!names.contains("commented"));
        assert!(!names.contains("in"));
    }
}
