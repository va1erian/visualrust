//! A hand-rolled HTTP/1.1 request parser for the required subset.

use std::collections::BTreeMap;
use std::io::{BufRead, Read};

use thiserror::Error;

use crate::headers::Headers;
use crate::method::Method;
use crate::request::Request;

/// Errors from reading a request off the wire.
#[derive(Debug, Error)]
pub enum ParseError {
    /// The request line was not `METHOD SP TARGET SP VERSION`.
    #[error("the request line is malformed")]
    MalformedRequestLine,
    /// The method is not one this server dispatches.
    #[error("unsupported HTTP method `{0}`")]
    UnsupportedMethod(String),
    /// The version was neither HTTP/1.1 nor HTTP/1.0.
    #[error("unsupported HTTP version `{0}`")]
    UnsupportedVersion(String),
    /// A header line had no `:` separator.
    #[error("malformed header line")]
    MalformedHeader,
    /// The request line plus headers exceeded the configured limit.
    #[error("the request head exceeds the {0}-byte limit")]
    HeadTooLarge(usize),
    /// `Content-Length` exceeded the configured limit.
    #[error("the request body exceeds the {0}-byte limit")]
    BodyTooLarge(usize),
    /// A request target or query string had bad percent-encoding.
    #[error("the request target has invalid percent-encoding")]
    InvalidEncoding,
    /// The connection ended before the declared body was read.
    #[error("the request body ended before the declared length")]
    UnexpectedEof,
    /// `Transfer-Encoding` was anything other than `identity`.
    #[error("chunked transfer-encoding is not supported")]
    UnsupportedTransferEncoding,
    /// The underlying socket failed.
    #[error("io error while reading the request: {0}")]
    Io(#[from] std::io::Error),
}

/// Caps that keep a malformed or hostile client from exhausting memory. Defaults
/// are generous for scripting but bounded.
#[derive(Debug, Clone, Copy)]
pub struct ParseLimits {
    /// Maximum bytes in the request line plus all headers.
    pub max_head: usize,
    /// Maximum `Content-Length` accepted.
    pub max_body: usize,
}

impl Default for ParseLimits {
    fn default() -> Self {
        Self {
            max_head: 64 * 1024,
            max_body: 8 * 1024 * 1024,
        }
    }
}

/// Reads one request from `reader`.
///
/// `Ok(None)` means the peer closed the connection cleanly before sending
/// anything, which is normal for a fresh keep-alive socket and must not be
/// reported as an error.
pub fn parse_request<R: BufRead>(
    reader: &mut R,
    limits: &ParseLimits,
) -> Result<Option<Request>, ParseError> {
    let Some(request_line) = read_line(reader, limits.max_head)? else {
        return Ok(None);
    };

    let mut parts = request_line.split(' ');
    let method_token = parts.next().ok_or(ParseError::MalformedRequestLine)?;
    let target = parts.next().ok_or(ParseError::MalformedRequestLine)?;
    let version = parts.next().ok_or(ParseError::MalformedRequestLine)?;
    if parts.next().is_some() {
        return Err(ParseError::MalformedRequestLine);
    }

    let method = Method::parse(method_token)
        .ok_or_else(|| ParseError::UnsupportedMethod(method_token.to_owned()))?;
    if version != "HTTP/1.1" && version != "HTTP/1.0" {
        return Err(ParseError::UnsupportedVersion(version.to_owned()));
    }

    let mut headers = Headers::new();
    let mut head_used = request_line.len() + 2;
    loop {
        let Some(line) = read_line(reader, limits.max_head)? else {
            return Err(ParseError::UnexpectedEof);
        };
        head_used += line.len() + 2;
        if head_used > limits.max_head {
            return Err(ParseError::HeadTooLarge(limits.max_head));
        }
        if line.is_empty() {
            break;
        }
        let (name, value) = line.split_once(':').ok_or(ParseError::MalformedHeader)?;
        headers.insert(name.trim(), value.trim());
    }

    if headers
        .get("transfer-encoding")
        .is_some_and(|value| !value.eq_ignore_ascii_case("identity"))
    {
        return Err(ParseError::UnsupportedTransferEncoding);
    }

    let content_length = match headers.get("content-length") {
        Some(raw) => raw
            .trim()
            .parse::<usize>()
            .map_err(|_| ParseError::MalformedHeader)?,
        None => 0,
    };
    if content_length > limits.max_body {
        return Err(ParseError::BodyTooLarge(limits.max_body));
    }

    let mut body = vec![0u8; content_length];
    if content_length > 0 {
        reader.read_exact(&mut body).map_err(|error| {
            if error.kind() == std::io::ErrorKind::UnexpectedEof {
                ParseError::UnexpectedEof
            } else {
                ParseError::Io(error)
            }
        })?;
    }

    let (path, query) = parse_target(target)?;
    Ok(Some(Request {
        method,
        path,
        params: BTreeMap::new(),
        query,
        headers,
        body,
    }))
}

/// Reads one CRLF- or LF-terminated line, rejecting anything longer than `max`.
///
/// The `max + 1` take is what makes the bound provable: if we read more than
/// `max` bytes without a newline, the line is over budget and further bytes are
/// left unread because the connection is discarded afterwards.
fn read_line<R: BufRead>(reader: &mut R, max: usize) -> Result<Option<String>, ParseError> {
    let mut buf = Vec::new();
    let read = reader
        .by_ref()
        .take(max as u64 + 1)
        .read_until(b'\n', &mut buf)?;
    if read == 0 {
        return Ok(None);
    }
    if buf.len() > max {
        return Err(ParseError::HeadTooLarge(max));
    }
    if buf.last() == Some(&b'\n') {
        buf.pop();
    }
    if buf.last() == Some(&b'\r') {
        buf.pop();
    }
    String::from_utf8(buf)
        .map(Some)
        .map_err(|_| ParseError::MalformedRequestLine)
}

fn parse_target(target: &str) -> Result<(String, BTreeMap<String, String>), ParseError> {
    let (raw_path, raw_query) = match target.split_once('?') {
        Some((path, query)) => (path, Some(query)),
        None => (target, None),
    };
    let path = decode(raw_path, false)?;
    let mut query = BTreeMap::new();
    if let Some(raw_query) = raw_query {
        for pair in raw_query.split('&') {
            if pair.is_empty() {
                continue;
            }
            let (key, value) = match pair.split_once('=') {
                Some((key, value)) => (key, value),
                None => (pair, ""),
            };
            query.insert(decode(key, true)?, decode(value, true)?);
        }
    }
    Ok((path, query))
}

/// Percent-decodes a component. `+` means space only in the query string, per
/// the `application/x-www-form-urlencoded` convention browsers use.
fn decode(input: &str, plus_as_space: bool) -> Result<String, ParseError> {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'%' => {
                let high = *bytes.get(index + 1).ok_or(ParseError::InvalidEncoding)?;
                let low = *bytes.get(index + 2).ok_or(ParseError::InvalidEncoding)?;
                out.push((hex(high)? << 4) | hex(low)?);
                index += 3;
            }
            b'+' if plus_as_space => {
                out.push(b' ');
                index += 1;
            }
            byte => {
                out.push(byte);
                index += 1;
            }
        }
    }
    String::from_utf8(out).map_err(|_| ParseError::InvalidEncoding)
}

fn hex(byte: u8) -> Result<u8, ParseError> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => Err(ParseError::InvalidEncoding),
    }
}

#[cfg(test)]
mod tests {
    use super::{ParseError, ParseLimits, parse_request};
    use crate::Method;
    use std::io::Cursor;

    fn parse(raw: &str) -> Result<Option<crate::Request>, ParseError> {
        parse_request(&mut Cursor::new(raw.as_bytes()), &ParseLimits::default())
    }

    #[test]
    fn parses_a_get_with_query_and_headers() {
        let raw = "GET /users/7?q=a+b&x=1 HTTP/1.1\r\nHost: example\r\nX-N: v\r\n\r\n";
        let request = parse(raw).expect("parse").expect("request");
        assert_eq!(request.method, Method::Get);
        assert_eq!(request.path, "/users/7");
        assert_eq!(request.query("q"), Some("a b"));
        assert_eq!(request.query("x"), Some("1"));
        assert_eq!(request.header("host"), Some("example"));
        assert!(request.body.is_empty());
    }

    #[test]
    fn parses_a_body() {
        let raw = "POST /echo HTTP/1.1\r\nContent-Length: 5\r\n\r\nhello";
        let request = parse(raw).expect("parse").expect("request");
        assert_eq!(request.method, Method::Post);
        assert_eq!(request.body, b"hello");
    }

    #[test]
    fn decodes_percent_escapes_in_the_path() {
        let raw = "GET /a%20b/%7Bc HTTP/1.1\r\n\r\n";
        let request = parse(raw).expect("parse").expect("request");
        assert_eq!(request.path, "/a b/{c");
    }

    #[test]
    fn clean_eof_is_not_a_request() {
        assert!(parse("").expect("parse").is_none());
    }

    #[test]
    fn rejects_unknown_method_and_version() {
        assert!(matches!(
            parse("TRACE / HTTP/1.1\r\n\r\n"),
            Err(ParseError::UnsupportedMethod(_))
        ));
        assert!(matches!(
            parse("GET / HTTP/2.0\r\n\r\n"),
            Err(ParseError::UnsupportedVersion(_))
        ));
    }

    #[test]
    fn rejects_oversized_head_and_body() {
        let limits = ParseLimits {
            max_head: 16,
            max_body: 4,
        };
        let raw = "GET /aaaaaaaaaaaaaaaaaaaa HTTP/1.1\r\n\r\n";
        assert!(matches!(
            parse_request(&mut Cursor::new(raw.as_bytes()), &limits),
            Err(ParseError::HeadTooLarge(_))
        ));
        let raw = "POST / HTTP/1.1\r\nContent-Length: 9\r\n\r\n";
        let body_limits = ParseLimits {
            max_head: 1024,
            max_body: 4,
        };
        assert!(matches!(
            parse_request(&mut Cursor::new(raw.as_bytes()), &body_limits),
            Err(ParseError::BodyTooLarge(_))
        ));
    }

    #[test]
    fn rejects_invalid_percent_encoding() {
        assert!(matches!(
            parse("GET /%zz HTTP/1.1\r\n\r\n"),
            Err(ParseError::InvalidEncoding)
        ));
    }

    #[test]
    fn rejects_chunked_transfer_encoding() {
        assert!(matches!(
            parse("POST / HTTP/1.1\r\nTransfer-Encoding: chunked\r\n\r\n"),
            Err(ParseError::UnsupportedTransferEncoding)
        ));
    }
}
