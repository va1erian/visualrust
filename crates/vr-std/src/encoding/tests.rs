//! Unit tests for the encoders, including known answers and rejections.

use super::*;

#[test]
fn base64_matches_the_rfc4648_vectors() {
    assert_eq!(base64_encode(""), "");
    assert_eq!(base64_encode("f"), "Zg==");
    assert_eq!(base64_encode("fo"), "Zm8=");
    assert_eq!(base64_encode("foo"), "Zm9v");
    assert_eq!(base64_encode("foob"), "Zm9vYg==");
    assert_eq!(base64_encode("fooba"), "Zm9vYmE=");
    assert_eq!(base64_encode("foobar"), "Zm9vYmFy");
}

#[test]
fn base64_round_trips_unicode_text() {
    let text = "VisualRust — héllo 😀";
    assert_eq!(base64_decode(&base64_encode(text)).unwrap(), text);
}

#[test]
fn base64_rejects_malformed_input() {
    assert_eq!(
        base64_decode("not base64!").unwrap_err(),
        EncodingError::InvalidBase64 {
            function: "base64_decode"
        }
    );
    assert_eq!(
        base64_decode("Zg").unwrap_err(),
        EncodingError::InvalidBase64 {
            function: "base64_decode"
        }
    );
}

#[test]
fn base64_rejects_non_utf8_payloads() {
    // "/w==" decodes to the single byte 0xFF, which is not UTF-8.
    assert_eq!(
        base64_decode("/w==").unwrap_err(),
        EncodingError::NotUtf8 {
            function: "base64_decode"
        }
    );
}

#[test]
fn url_encode_escapes_reserved_bytes() {
    assert_eq!(url_encode("a b+c/d?"), "a%20b%2Bc%2Fd%3F");
    assert_eq!(url_encode("safe-._~"), "safe-._~");
    assert_eq!(url_encode("héllo"), "h%C3%A9llo");
    assert_eq!(url_encode(""), "");
}

#[test]
fn url_round_trips_arbitrary_text() {
    let text = "q=1&x=ä b/c?d#e";
    assert_eq!(url_decode(&url_encode(text)).unwrap(), text);
}

#[test]
fn url_decode_rejects_bad_percent_escapes() {
    assert_eq!(
        url_decode("%").unwrap_err(),
        EncodingError::InvalidPercentEscape {
            function: "url_decode",
            position: 0
        }
    );
    assert_eq!(
        url_decode("a%zzb").unwrap_err(),
        EncodingError::InvalidPercentEscape {
            function: "url_decode",
            position: 1
        }
    );
    assert_eq!(
        url_decode("abc%4").unwrap_err(),
        EncodingError::InvalidPercentEscape {
            function: "url_decode",
            position: 3
        }
    );
}

#[test]
fn url_decode_rejects_non_utf8_percent_sequences() {
    // %FF is a valid escape but not a valid UTF-8 byte.
    assert_eq!(
        url_decode("%FF").unwrap_err(),
        EncodingError::NotUtf8 {
            function: "url_decode"
        }
    );
}
