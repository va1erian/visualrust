//! Bounded `multipart/form-data` parsing.
//!
//! This is deliberately a parser, not a framework: it understands the RFC 7578
//! subset browsers send (one boundary, CRLF line endings, quoted disposition
//! parameters) and rejects anything else with a typed error. The caller passes
//! a byte cap so a hostile upload cannot exhaust memory.

use thiserror::Error;

/// Errors from parsing a multipart body.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum MultipartError {
    /// `Content-Type` was not `multipart/form-data`.
    #[error("the request is not multipart/form-data")]
    NotMultipart,
    /// The `Content-Type` had no `boundary` parameter.
    #[error("the multipart request has no boundary")]
    MissingBoundary,
    /// The body exceeded the caller's cap.
    #[error("the multipart body exceeds the {0}-byte limit")]
    BodyTooLarge(usize),
    /// The body did not follow the multipart grammar.
    #[error("the multipart body is malformed: {0}")]
    Malformed(String),
}

/// One parsed part.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Part {
    /// The form field name from `Content-Disposition`.
    pub name: String,
    /// The client-supplied filename, for file fields.
    pub filename: Option<String>,
    /// The part's `Content-Type`, if it declared one.
    pub content_type: Option<String>,
    /// The raw part bytes.
    pub data: Vec<u8>,
}

/// Parses `body` against the `boundary` in `content_type`.
///
/// `max_bytes` caps the whole body, so the returned parts are bounded by
/// construction; a body over the cap is rejected before any parsing.
pub fn parse(
    body: &[u8],
    content_type: &str,
    max_bytes: usize,
) -> Result<Vec<Part>, MultipartError> {
    let boundary = boundary_of(content_type)?;
    if body.len() > max_bytes {
        return Err(MultipartError::BodyTooLarge(max_bytes));
    }
    let marker = format!("--{boundary}").into_bytes();
    let separator = format!("\r\n--{boundary}").into_bytes();

    let start = find(body, &marker)
        .ok_or_else(|| MultipartError::Malformed("missing opening boundary".to_owned()))?;
    let mut rest = &body[start + marker.len()..];
    let mut parts = Vec::new();
    loop {
        // A closing delimiter is the marker followed by `--`.
        if rest.starts_with(b"--") {
            break;
        }
        let Some(after_crlf) = rest.strip_prefix(b"\r\n") else {
            return Err(MultipartError::Malformed(
                "boundary not followed by CRLF".to_owned(),
            ));
        };
        rest = after_crlf;

        let header_end = find(rest, b"\r\n\r\n")
            .ok_or_else(|| MultipartError::Malformed("part headers are unterminated".to_owned()))?;
        let header_block = std::str::from_utf8(&rest[..header_end])
            .map_err(|_| MultipartError::Malformed("part headers are not UTF-8".to_owned()))?;
        let after_headers = &rest[header_end + 4..];

        let (data, next) = match find(after_headers, &separator) {
            Some(index) => (
                &after_headers[..index],
                &after_headers[index + separator.len()..],
            ),
            None => {
                return Err(MultipartError::Malformed(
                    "part is not terminated by a boundary".to_owned(),
                ));
            }
        };
        parts.push(build_part(header_block, data)?);
        rest = next;
    }
    Ok(parts)
}

fn build_part(header_block: &str, data: &[u8]) -> Result<Part, MultipartError> {
    let mut name = None;
    let mut filename = None;
    let mut content_type = None;
    for line in header_block.split("\r\n") {
        let Some((header, value)) = line.split_once(':') else {
            continue;
        };
        match header.trim().to_ascii_lowercase().as_str() {
            "content-disposition" => {
                name = disposition_param(value, "name");
                filename = disposition_param(value, "filename");
            }
            "content-type" => content_type = Some(value.trim().to_owned()),
            _ => {}
        }
    }
    let name = name.ok_or_else(|| MultipartError::Malformed("part has no name".to_owned()))?;
    Ok(Part {
        name,
        filename,
        content_type,
        data: data.to_vec(),
    })
}

/// Pulls one quoted parameter out of a `Content-Disposition` value.
fn disposition_param(value: &str, key: &str) -> Option<String> {
    let mut segments = value.split(';');
    segments.next();
    for segment in segments {
        let Some((name, raw)) = segment.trim().split_once('=') else {
            continue;
        };
        if !name.trim().eq_ignore_ascii_case(key) {
            continue;
        }
        let raw = raw.trim();
        let unquoted = raw
            .strip_prefix('"')
            .and_then(|rest| rest.strip_suffix('"'))
            .unwrap_or(raw);
        return Some(unquoted.replace("\\\"", "\"").replace("\\\\", "\\"));
    }
    None
}

fn boundary_of(content_type: &str) -> Result<String, MultipartError> {
    let mut segments = content_type.split(';');
    let kind = segments.next().map(str::trim).unwrap_or_default();
    if !kind.eq_ignore_ascii_case("multipart/form-data") {
        return Err(MultipartError::NotMultipart);
    }
    for segment in segments {
        let Some((name, raw)) = segment.trim().split_once('=') else {
            continue;
        };
        if !name.trim().eq_ignore_ascii_case("boundary") {
            continue;
        }
        let raw = raw.trim();
        let boundary = raw
            .strip_prefix('"')
            .and_then(|rest| rest.strip_suffix('"'))
            .unwrap_or(raw);
        if boundary.is_empty() {
            return Err(MultipartError::MissingBoundary);
        }
        return Ok(boundary.to_owned());
    }
    Err(MultipartError::MissingBoundary)
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

#[cfg(test)]
mod tests {
    use super::{MultipartError, parse};

    fn body(boundary: &str) -> Vec<u8> {
        format!(
            "--{boundary}\r\n\
             Content-Disposition: form-data; name=\"field\"\r\n\
             \r\n\
             value\r\n\
             --{boundary}\r\n\
             Content-Disposition: form-data; name=\"file\"; filename=\"a.txt\"\r\n\
             Content-Type: text/plain\r\n\
             \r\n\
             hello file\r\n\
             --{boundary}--\r\n"
        )
        .into_bytes()
    }

    #[test]
    fn parses_named_and_file_parts() {
        let parts = parse(&body("xyz"), "multipart/form-data; boundary=xyz", 4096).expect("parse");
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0].name, "field");
        assert_eq!(parts[0].data, b"value");
        assert_eq!(parts[1].name, "file");
        assert_eq!(parts[1].filename.as_deref(), Some("a.txt"));
        assert_eq!(parts[1].content_type.as_deref(), Some("text/plain"));
        assert_eq!(parts[1].data, b"hello file");
    }

    #[test]
    fn rejects_a_body_over_the_cap() {
        let raw = body("xyz");
        let cap = raw.len() - 1;
        assert_eq!(
            parse(&raw, "multipart/form-data; boundary=xyz", cap),
            Err(MultipartError::BodyTooLarge(cap))
        );
    }

    #[test]
    fn rejects_missing_boundary_and_non_multipart() {
        assert_eq!(
            parse(b"body", "multipart/form-data", 1024),
            Err(MultipartError::MissingBoundary)
        );
        assert_eq!(
            parse(b"body", "application/json", 1024),
            Err(MultipartError::NotMultipart)
        );
    }
}
