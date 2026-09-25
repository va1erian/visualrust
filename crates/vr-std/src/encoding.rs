//! base64 and URL percent-encoding over UTF-8 text.
//!
//! Dyon only has strings, so [`base64_decode`] and [`url_decode`] return a
//! UTF-8 string and reject anything that does not decode to valid UTF-8 rather
//! than handing back lossy bytes. The URL form uses the RFC 3986 unreserved set
//! (`A-Z a-z 0-9 - . _ ~`), so [`url_encode`] and [`url_decode`] round-trip.
//!
//! [`url_decode`] is stricter than the `percent-encoding` crate on purpose: a
//! lone `%` or `%zz` is a typed [`EncodingError::InvalidPercentEscape`] instead
//! of being passed through verbatim, which is what a script author expects from
//! a decoder.

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use percent_encoding::{AsciiSet, NON_ALPHANUMERIC, percent_decode_str, utf8_percent_encode};

use crate::error::EncodingError;

/// RFC 3986 unreserved characters stay literal; everything else is escaped.
const URL_ENCODE_SET: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'-')
    .remove(b'.')
    .remove(b'_')
    .remove(b'~');

/// Standard base64 (RFC 4648) of `text`'s UTF-8 bytes.
pub fn base64_encode(text: &str) -> String {
    STANDARD.encode(text.as_bytes())
}

/// Decodes standard base64 and requires the result to be UTF-8.
pub fn base64_decode(text: &str) -> Result<String, EncodingError> {
    let bytes = STANDARD
        .decode(text)
        .map_err(|_| EncodingError::InvalidBase64 {
            function: "base64_decode",
        })?;
    String::from_utf8(bytes).map_err(|_| EncodingError::NotUtf8 {
        function: "base64_decode",
    })
}

/// Percent-encodes every byte outside the RFC 3986 unreserved set.
pub fn url_encode(text: &str) -> String {
    utf8_percent_encode(text, URL_ENCODE_SET).to_string()
}

/// Decodes URL percent-encoding, rejecting malformed escapes and non-UTF-8.
pub fn url_decode(text: &str) -> Result<String, EncodingError> {
    validate_percent_escapes(text, "url_decode")?;
    percent_decode_str(text)
        .decode_utf8()
        .map(|cow| cow.into_owned())
        .map_err(|_| EncodingError::NotUtf8 {
            function: "url_decode",
        })
}

/// Checks that every `%` starts a two-digit hexadecimal escape.
///
/// `percent_decode_str` is lenient and would echo a bad escape unchanged; a
/// decoder should instead report it.
fn validate_percent_escapes(text: &str, function: &'static str) -> Result<(), EncodingError> {
    let bytes = text.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            let is_escape = bytes
                .get(index + 1)
                .is_some_and(|byte| byte.is_ascii_hexdigit())
                && bytes
                    .get(index + 2)
                    .is_some_and(|byte| byte.is_ascii_hexdigit());
            if !is_escape {
                return Err(EncodingError::InvalidPercentEscape {
                    function,
                    position: index,
                });
            }
            index += 3;
        } else {
            index += 1;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
