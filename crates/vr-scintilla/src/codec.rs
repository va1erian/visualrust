//! Pure encoding helpers for the pointer/length `SCI_*` messages.
//!
//! Scintilla takes NUL-terminated UTF-8 for `SCI_SETTEXT`, `SCI_MARGINSETTEXT`
//! and friends, and writes NUL-terminated UTF-8 for `SCI_GETTEXT`. Keeping the
//! byte handling here means the unsafe module only has to pass a pointer and a
//! length.

use crate::{Error, Result};

/// Encodes `text` as UTF-8 with a trailing NUL, the form Scintilla expects for
/// its string messages.
pub(crate) fn encode_c_string(text: &str) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(text.len() + 1);
    bytes.extend_from_slice(text.as_bytes());
    bytes.push(0);
    bytes
}

/// Decodes a NUL-terminated UTF-8 buffer, stopping at the first NUL and
/// ignoring anything after it.
pub(crate) fn decode_c_string(buffer: &[u8]) -> Result<String> {
    let end = buffer
        .iter()
        .position(|&byte| byte == 0)
        .unwrap_or(buffer.len());
    String::from_utf8(buffer[..end].to_vec()).map_err(|_| Error::TextEncoding)
}

/// Run-length encodes per-character style numbers into `(length, style)` runs,
/// as consumed by repeated `SCI_SETSTYLING` calls.
pub(crate) fn style_runs(styles: &[u8]) -> Vec<(usize, u8)> {
    let mut runs = Vec::new();
    let mut index = 0;
    while index < styles.len() {
        let style = styles[index];
        let mut length = 1;
        while index + length < styles.len() && styles[index + length] == style {
            length += 1;
        }
        runs.push((length, style));
        index += length;
    }
    runs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_text_with_a_trailing_nul() {
        assert_eq!(encode_c_string("hi"), b"hi\0");
        assert_eq!(encode_c_string(""), b"\0");
    }

    #[test]
    fn decodes_only_up_to_the_first_nul() {
        assert_eq!(decode_c_string(b"hi\0junk").ok().as_deref(), Some("hi"));
        assert_eq!(decode_c_string(b"hi").ok().as_deref(), Some("hi"));
        assert_eq!(decode_c_string(b"\0").ok().as_deref(), Some(""));
    }

    #[test]
    fn rejects_invalid_utf8() {
        assert!(matches!(
            decode_c_string(&[0xFF, 0xFE, 0x00]),
            Err(Error::TextEncoding)
        ));
    }

    #[test]
    fn groups_equal_style_neighbours_into_runs() {
        assert_eq!(
            style_runs(&[1, 1, 2, 2, 2, 3]),
            vec![(2, 1), (3, 2), (1, 3)]
        );
        assert!(style_runs(&[]).is_empty());
    }

    #[test]
    fn round_trips_unicode() {
        let text = "fn main() { /* \u{2713} */ }";
        let bytes = encode_c_string(text);
        assert_eq!(decode_c_string(&bytes).ok().as_deref(), Some(text));
    }
}
