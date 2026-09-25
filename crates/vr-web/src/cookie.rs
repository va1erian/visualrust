//! HTTP cookies: parsing `Cookie` request headers and formatting `Set-Cookie`.
//!
//! Request cookies arrive as one header, `name=value; name2=value2`; response
//! cookies need their own header to carry attributes. Response cookies are
//! therefore kept separately from plain headers and emitted as repeated
//! `Set-Cookie` lines by [`Response::to_http`](crate::Response).

use std::fmt::Write as _;

use thiserror::Error;

use crate::headers::Headers;

/// Errors from building a [`Cookie`].
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CookieError {
    /// The name was empty or contained a character outside the HTTP token set.
    #[error("cookie name is empty or contains an invalid character")]
    InvalidName,
    /// The value contained a control character or one of `; , " \`.
    #[error("cookie value contains an invalid character")]
    InvalidValue,
}

/// The `SameSite` attribute.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SameSite {
    /// Sent only for same-site requests.
    Strict,
    /// Sent for top-level cross-site navigation (the browser default).
    Lax,
    /// Sent on every request; only safe for cookies a server can trust.
    None,
}

impl SameSite {
    /// The attribute spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            SameSite::Strict => "Strict",
            SameSite::Lax => "Lax",
            SameSite::None => "None",
        }
    }
}

/// One response cookie with its attributes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cookie {
    name: String,
    value: String,
    path: Option<String>,
    max_age: Option<i64>,
    expires: Option<String>,
    http_only: bool,
    secure: bool,
    same_site: Option<SameSite>,
}

impl Cookie {
    /// Builds a cookie, rejecting names and values that would break the header.
    pub fn new(name: impl Into<String>, value: impl Into<String>) -> Result<Self, CookieError> {
        let name = name.into();
        let value = value.into();
        if !is_token(&name) {
            return Err(CookieError::InvalidName);
        }
        if !is_cookie_value(&value) {
            return Err(CookieError::InvalidValue);
        }
        Ok(Self {
            name,
            value,
            path: None,
            max_age: None,
            expires: None,
            http_only: false,
            secure: false,
            same_site: None,
        })
    }

    /// The cookie name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The cookie value.
    pub fn value(&self) -> &str {
        &self.value
    }

    /// Sets `Path`. An unsafe value is dropped rather than risking header
    /// injection through a `;` or a control character.
    pub fn path(mut self, path: impl Into<String>) -> Self {
        let path = path.into();
        if is_attribute_safe(&path) {
            self.path = Some(path);
        }
        self
    }

    /// Sets `Max-Age`; a negative age tells the client to delete the cookie.
    pub fn max_age(mut self, seconds: i64) -> Self {
        self.max_age = Some(seconds);
        self
    }

    /// Sets `Expires` from an already-formatted HTTP-date string.
    pub fn expires(mut self, date: impl Into<String>) -> Self {
        let date = date.into();
        if is_attribute_safe(&date) {
            self.expires = Some(date);
        }
        self
    }

    /// Adds `HttpOnly`, hiding the cookie from scripts.
    pub fn http_only(mut self) -> Self {
        self.http_only = true;
        self
    }

    /// Adds `Secure`.
    pub fn secure(mut self) -> Self {
        self.secure = true;
        self
    }

    /// Sets `SameSite`.
    pub fn same_site(mut self, same_site: SameSite) -> Self {
        self.same_site = Some(same_site);
        self
    }

    /// Formats the `Set-Cookie` header value.
    pub fn to_set_cookie(&self) -> String {
        let mut out = String::with_capacity(64);
        out.push_str(&self.name);
        out.push('=');
        out.push_str(&self.value);
        if let Some(path) = &self.path {
            let _ = write!(out, "; Path={path}");
        }
        if let Some(max_age) = self.max_age {
            let _ = write!(out, "; Max-Age={max_age}");
        }
        if let Some(expires) = &self.expires {
            let _ = write!(out, "; Expires={expires}");
        }
        if self.http_only {
            out.push_str("; HttpOnly");
        }
        if self.secure {
            out.push_str("; Secure");
        }
        if let Some(same_site) = self.same_site {
            let _ = write!(out, "; SameSite={}", same_site.as_str());
        }
        out
    }
}

/// Parses a `Cookie` request header into `(name, value)` pairs.
pub fn parse_cookie_header(header: &str) -> Vec<(String, String)> {
    let mut cookies = Vec::new();
    for pair in header.split(';') {
        let pair = pair.trim();
        let Some((name, value)) = pair.split_once('=') else {
            continue;
        };
        let name = name.trim();
        if name.is_empty() {
            continue;
        }
        cookies.push((name.to_owned(), unquote(value)));
    }
    cookies
}

/// Reads the `Cookie` header, if present, into pairs.
pub fn cookies_from(headers: &Headers) -> Vec<(String, String)> {
    headers
        .get("cookie")
        .map(parse_cookie_header)
        .unwrap_or_default()
}

fn unquote(value: &str) -> String {
    let trimmed = value.trim();
    match trimmed
        .strip_prefix('"')
        .and_then(|rest| rest.strip_suffix('"'))
    {
        Some(inner) => inner.replace("\\\"", "\"").replace("\\\\", "\\"),
        None => trimmed.to_owned(),
    }
}

/// RFC 7230 token: the only characters valid in a cookie name.
fn is_token(name: &str) -> bool {
    !name.is_empty()
        && name.bytes().all(|byte| {
            byte.is_ascii_alphanumeric()
                || matches!(
                    byte,
                    b'!' | b'#'
                        | b'$'
                        | b'%'
                        | b'&'
                        | b'\''
                        | b'*'
                        | b'+'
                        | b'-'
                        | b'.'
                        | b'^'
                        | b'_'
                        | b'`'
                        | b'|'
                        | b'~'
                )
        })
}

/// Printable ASCII without the separators that would confuse a cookie parser.
fn is_cookie_value(value: &str) -> bool {
    value
        .bytes()
        .all(|byte| (0x21..=0x7E).contains(&byte) && !matches!(byte, b';' | b',' | b'"' | b'\\'))
}

fn is_attribute_safe(value: &str) -> bool {
    !value.is_empty()
        && !value
            .bytes()
            .any(|byte| byte == b';' || byte.is_ascii_control())
}

#[cfg(test)]
mod tests {
    use super::{Cookie, CookieError, SameSite, parse_cookie_header};

    #[test]
    fn parses_a_request_cookie_header() {
        let parsed = parse_cookie_header("a=1; b=two; empty=; flag");
        assert_eq!(
            parsed,
            vec![
                ("a".to_owned(), "1".to_owned()),
                ("b".to_owned(), "two".to_owned()),
                ("empty".to_owned(), String::new()),
            ]
        );
    }

    #[test]
    fn formats_set_cookie_attributes() {
        let cookie = Cookie::new("vr_session", "abc123")
            .expect("cookie")
            .path("/")
            .max_age(3600)
            .http_only()
            .secure()
            .same_site(SameSite::Lax);
        assert_eq!(
            cookie.to_set_cookie(),
            "vr_session=abc123; Path=/; Max-Age=3600; HttpOnly; Secure; SameSite=Lax"
        );
    }

    #[test]
    fn drops_unsafe_attributes_and_rejects_bad_names() {
        let cookie = Cookie::new("x", "y")
            .expect("cookie")
            .path("/a;b\r\nInjected: 1");
        assert_eq!(cookie.to_set_cookie(), "x=y");
        assert_eq!(Cookie::new("bad name", "y"), Err(CookieError::InvalidName));
        assert_eq!(Cookie::new("x", "a\r\nb"), Err(CookieError::InvalidValue));
    }
}
