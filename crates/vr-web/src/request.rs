//! The request message handed to handlers.

use std::collections::BTreeMap;

use crate::headers::Headers;
use crate::method::Method;

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
}
