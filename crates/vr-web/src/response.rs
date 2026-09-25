//! The response message produced by handlers.

use std::fmt::{self, Write as _};

use crate::cookie::Cookie;
use crate::headers::Headers;

/// An HTTP status code.
///
/// A newtype rather than a bare `u16` so handlers can only build well-known
/// responses through the constants, while still allowing a custom code through
/// [`StatusCode::from_u16`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StatusCode(u16);

impl StatusCode {
    /// `200 OK`.
    pub const OK: Self = Self(200);
    /// `204 No Content`.
    pub const NO_CONTENT: Self = Self(204);
    /// `301 Moved Permanently`.
    pub const MOVED_PERMANENTLY: Self = Self(301);
    /// `302 Found`, the default for [`Response::redirect`].
    pub const FOUND: Self = Self(302);
    /// `303 See Other`.
    pub const SEE_OTHER: Self = Self(303);
    /// `400 Bad Request`.
    pub const BAD_REQUEST: Self = Self(400);
    /// `404 Not Found`.
    pub const NOT_FOUND: Self = Self(404);
    /// `405 Method Not Allowed`.
    pub const METHOD_NOT_ALLOWED: Self = Self(405);
    /// `411 Length Required`.
    pub const LENGTH_REQUIRED: Self = Self(411);
    /// `413 Payload Too Large`.
    pub const PAYLOAD_TOO_LARGE: Self = Self(413);
    /// `500 Internal Server Error`.
    pub const INTERNAL_SERVER_ERROR: Self = Self(500);

    /// Wraps an arbitrary status code.
    pub fn from_u16(code: u16) -> Self {
        Self(code)
    }

    /// The numeric code.
    pub fn as_u16(self) -> u16 {
        self.0
    }

    /// The canonical reason phrase; unknown codes report `"Unknown"` rather than
    /// omitting the phrase, because HTTP/1.1 requires one.
    pub fn reason(self) -> &'static str {
        match self.0 {
            200 => "OK",
            204 => "No Content",
            301 => "Moved Permanently",
            302 => "Found",
            303 => "See Other",
            400 => "Bad Request",
            404 => "Not Found",
            405 => "Method Not Allowed",
            411 => "Length Required",
            413 => "Payload Too Large",
            500 => "Internal Server Error",
            _ => "Unknown",
        }
    }
}

impl fmt::Display for StatusCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.0, self.reason())
    }
}

/// A response message.
#[derive(Debug, Clone)]
pub struct Response {
    /// The status code.
    pub status: StatusCode,
    /// Response headers; `content-length` and `connection` are added on write.
    pub headers: Headers,
    /// Body bytes.
    pub body: Vec<u8>,
    /// Cookies to emit as repeated `set-cookie` headers.
    pub cookies: Vec<Cookie>,
}

impl Response {
    /// A response with the given status and no body.
    pub fn new(status: StatusCode) -> Self {
        Self {
            status,
            headers: Headers::new(),
            body: Vec::new(),
            cookies: Vec::new(),
        }
    }

    /// A `text/plain` response.
    pub fn text(status: StatusCode, body: impl Into<String>) -> Self {
        let mut response = Self::new(status);
        response
            .headers
            .insert("content-type", "text/plain; charset=utf-8");
        response.body = body.into().into_bytes();
        response
    }

    /// A `text/html` response.
    pub fn html(status: StatusCode, body: impl Into<String>) -> Self {
        let mut response = Self::new(status);
        response
            .headers
            .insert("content-type", "text/html; charset=utf-8");
        response.body = body.into().into_bytes();
        response
    }

    /// A JSON response. Serialisation failure becomes a `500` rather than a
    /// panic, so a value with a bad `Serialize` impl cannot take a worker down.
    pub fn json<T: serde::Serialize>(status: StatusCode, value: &T) -> Self {
        match serde_json::to_vec(value) {
            Ok(body) => {
                let mut response = Self::new(status);
                response.headers.insert("content-type", "application/json");
                response.body = body;
                response
            }
            Err(_) => Self::text(
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to serialise the JSON response",
            ),
        }
    }

    /// A response of arbitrary bytes under a caller-supplied content type.
    pub fn bytes(
        status: StatusCode,
        content_type: impl Into<String>,
        body: impl Into<Vec<u8>>,
    ) -> Self {
        let mut response = Self::new(status);
        response.headers.insert("content-type", content_type);
        response.body = body.into();
        response
    }

    /// A `302 Found` redirect to `location`.
    pub fn redirect(location: impl Into<String>) -> Self {
        Self::new(StatusCode::FOUND).with_header("location", location)
    }

    /// Replaces the status, returning `self` for chaining.
    pub fn with_status(mut self, status: StatusCode) -> Self {
        self.status = status;
        self
    }

    /// Adds or replaces a header, returning `self` for chaining.
    pub fn with_header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(name, value);
        self
    }

    /// Replaces the body, returning `self` for chaining.
    pub fn with_body(mut self, body: impl Into<Vec<u8>>) -> Self {
        self.body = body.into();
        self
    }

    /// Adds a `set-cookie` header, returning `self` for chaining.
    pub fn with_cookie(mut self, cookie: Cookie) -> Self {
        self.cookies.push(cookie);
        self
    }

    /// Serialises the response to HTTP/1.1 bytes.
    ///
    /// The server serves one request per connection, so `connection: close` is
    /// always emitted and `content-length` is derived from the body unless a
    /// handler already set it.
    pub(crate) fn to_http(&self) -> Vec<u8> {
        let mut head = String::with_capacity(128);
        let _ = write!(
            head,
            "HTTP/1.1 {} {}\r\n",
            self.status.as_u16(),
            self.status.reason()
        );
        for (name, value) in self.headers.iter() {
            head.push_str(name);
            head.push_str(": ");
            head.push_str(value);
            head.push_str("\r\n");
        }
        // Cookies are emitted separately because `Headers` keeps one entry per
        // name, while a response may legitimately set several cookies.
        for cookie in &self.cookies {
            head.push_str("set-cookie: ");
            head.push_str(&cookie.to_set_cookie());
            head.push_str("\r\n");
        }
        if self.headers.get("content-length").is_none() {
            head.push_str("content-length: ");
            head.push_str(&self.body.len().to_string());
            head.push_str("\r\n");
        }
        head.push_str("connection: close\r\n\r\n");

        let mut bytes = head.into_bytes();
        bytes.extend_from_slice(&self.body);
        bytes
    }
}

#[cfg(test)]
mod tests {
    use super::{Response, StatusCode};
    use crate::cookie::Cookie;

    #[test]
    fn text_sets_content_type_and_length() {
        let bytes = Response::text(StatusCode::OK, "hi").to_http();
        let text = String::from_utf8(bytes).expect("utf8");
        assert!(text.starts_with("HTTP/1.1 200 OK\r\n"));
        assert!(text.contains("content-type: text/plain; charset=utf-8\r\n"));
        assert!(text.contains("content-length: 2\r\n"));
        assert!(text.ends_with("\r\n\r\nhi"));
    }

    #[test]
    fn handler_content_length_wins() {
        let bytes = Response::new(StatusCode::NO_CONTENT)
            .with_header("content-length", "0")
            .to_http();
        let text = String::from_utf8(bytes).expect("utf8");
        assert_eq!(text.matches("content-length").count(), 1);
    }

    #[test]
    fn json_serialises_and_sets_the_content_type() {
        let value = serde_json::json!({"name": "ada", "id": 7});
        let response = Response::json(StatusCode::OK, &value);
        assert_eq!(
            response.headers.get("content-type"),
            Some("application/json")
        );
        assert_eq!(response.body, br#"{"id":7,"name":"ada"}"#);
    }

    #[test]
    fn redirect_sets_the_location_and_status() {
        let response = Response::redirect("/login");
        assert_eq!(response.status, StatusCode::FOUND);
        assert_eq!(response.headers.get("location"), Some("/login"));
    }

    #[test]
    fn html_and_bytes_set_their_content_types() {
        assert_eq!(
            Response::html(StatusCode::OK, "<p>")
                .headers
                .get("content-type"),
            Some("text/html; charset=utf-8")
        );
        assert_eq!(
            Response::bytes(StatusCode::OK, "image/png", vec![1, 2])
                .headers
                .get("content-type"),
            Some("image/png")
        );
    }

    #[test]
    fn emits_one_set_cookie_header_per_cookie() {
        let cookie = Cookie::new("a", "1").expect("cookie").path("/");
        let response = Response::new(StatusCode::OK).with_cookie(cookie);
        let text = String::from_utf8(response.to_http()).expect("utf8");
        assert!(text.contains("set-cookie: a=1; Path=/\r\n"), "{text}");
    }
}
