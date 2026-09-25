//! The HTTP methods the router understands.

use std::fmt;

/// A request method. Unknown methods are rejected while parsing, so the router
/// never has to reason about tokens it cannot dispatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Method {
    Get,
    Post,
    Put,
    Delete,
    Patch,
    Head,
    Options,
}

impl Method {
    /// Every method the parser accepts.
    pub const ALL: [Method; 7] = [
        Method::Get,
        Method::Post,
        Method::Put,
        Method::Delete,
        Method::Patch,
        Method::Head,
        Method::Options,
    ];

    /// The wire token, as it appears in a request line.
    pub fn as_str(self) -> &'static str {
        match self {
            Method::Get => "GET",
            Method::Post => "POST",
            Method::Put => "PUT",
            Method::Delete => "DELETE",
            Method::Patch => "PATCH",
            Method::Head => "HEAD",
            Method::Options => "OPTIONS",
        }
    }

    /// Parses a wire token, returning `None` for methods this server cannot
    /// dispatch. Returning `Option` keeps the caller responsible for turning the
    /// miss into a typed parse error.
    pub fn parse(token: &str) -> Option<Self> {
        match token {
            "GET" => Some(Method::Get),
            "POST" => Some(Method::Post),
            "PUT" => Some(Method::Put),
            "DELETE" => Some(Method::Delete),
            "PATCH" => Some(Method::Patch),
            "HEAD" => Some(Method::Head),
            "OPTIONS" => Some(Method::Options),
            _ => None,
        }
    }
}

impl fmt::Display for Method {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::Method;

    #[test]
    fn round_trips_every_known_method() {
        for method in Method::ALL {
            assert_eq!(Method::parse(method.as_str()), Some(method));
        }
    }

    #[test]
    fn rejects_unknown_tokens() {
        assert_eq!(Method::parse("TRACE"), None);
        assert_eq!(Method::parse("get"), None);
    }
}
