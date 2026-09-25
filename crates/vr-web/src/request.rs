//! The request message handed to handlers.

use std::collections::BTreeMap;

use crate::cookie;
use crate::headers::Headers;
use crate::method::Method;
use crate::multipart::{self, MultipartError, Part};

/// A self-contained view of one incoming HTTP request.
///
/// The parser fills in everything except `params`; the router fills `params`
/// once it has matched a route, so a handler sees both the matched captures and
/// the raw request fields in one value. `BTreeMap` keeps iteration deterministic,
/// which makes tests and Dyon bridging predictable.
#[derive(Debug, Clone)]
pub struct Request {
    /// The request method.
    pub method: Method,
    /// The percent-decoded path, without the query string.
    pub path: String,
    /// Captures from the matched route pattern, keyed by parameter name.
    pub params: BTreeMap<String, String>,
    /// Percent-decoded query parameters.
    pub query: BTreeMap<String, String>,
    /// Request headers.
    pub headers: Headers,
    /// Raw body bytes, exactly `Content-Length` long.
    pub body: Vec<u8>,
}

impl Request {
    /// Builds an empty-body request; used by the parser and by tests.
    pub fn new(method: Method, path: impl Into<String>) -> Self {
        Self {
            method,
            path: path.into(),
            params: BTreeMap::new(),
            query: BTreeMap::new(),
            headers: Headers::new(),
            body: Vec::new(),
        }
    }

    /// A header value, or `None`.
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(name)
    }

    /// A query parameter, or `None`.
    pub fn query(&self, name: &str) -> Option<&str> {
        self.query.get(name).map(String::as_str)
    }

    /// A route parameter captured by the matched pattern, or `None`.
    pub fn param(&self, name: &str) -> Option<&str> {
        self.params.get(name).map(String::as_str)
    }

    /// The raw `content-type` header, or `None`.
    pub fn content_type(&self) -> Option<&str> {
        self.headers.get("content-type")
    }

    /// The parsed `Cookie` header as `(name, value)` pairs.
    pub fn cookies(&self) -> Vec<(String, String)> {
        cookie::cookies_from(&self.headers)
    }

    /// One request cookie by name, or `None`.
    pub fn cookie(&self, name: &str) -> Option<String> {
        self.cookies()
            .into_iter()
            .find(|(cookie_name, _)| cookie_name == name)
            .map(|(_, value)| value)
    }

    /// Parses `multipart/form-data` parts, rejecting a body over `max_bytes`.
    pub fn multipart(&self, max_bytes: usize) -> Result<Vec<Part>, MultipartError> {
        let content_type = self.content_type().ok_or(MultipartError::NotMultipart)?;
        multipart::parse(&self.body, content_type, max_bytes)
    }

    /// The body decoded lossily as UTF-8, for text and form payloads.
    pub fn body_text(&self) -> std::borrow::Cow<'_, str> {
        String::from_utf8_lossy(&self.body)
    }
}
