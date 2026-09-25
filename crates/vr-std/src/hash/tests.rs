//! Known-answer tests for the digests and HMAC.

use super::*;

#[test]
fn md5_matches_known_vectors() {
    assert_eq!(md5(""), "d41d8cd98f00b204e9800998ecf8427e");
    assert_eq!(md5("abc"), "900150983cd24fb0d6963f7d28e17f72");
    assert_eq!(
        md5("The quick brown fox jumps over the lazy dog"),
        "9e107d9d372bb6826bd81d3542a419d6"
    );
}

#[test]
fn sha1_matches_known_vectors() {
    assert_eq!(sha1(""), "da39a3ee5e6b4b0d3255bfef95601890afd80709");
    assert_eq!(sha1("abc"), "a9993e364706816aba3e25717850c26c9cd0d89d");
    assert_eq!(
        sha1("The quick brown fox jumps over the lazy dog"),
        "2fd4e1c67a2d28fced849ee1bb76e7391b93eb12"
    );
}

#[test]
fn sha256_matches_known_vectors() {
    assert_eq!(
        sha256(""),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
    assert_eq!(
        sha256("abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    assert_eq!(
        sha256("The quick brown fox jumps over the lazy dog"),
        "d7a8fbb307d7809469ca9abcb0082e4f8d5651e46d3cdb762d02d0bf37c9e592"
    );
}

#[test]
fn hmac_sha256_matches_rfc4231_case_1() {
    // RFC 4231 test case 1: a 20-byte 0x0b key over "Hi There".
    let key = "\u{0b}".repeat(20);
    assert_eq!(
        hmac_sha256(&key, "Hi There").unwrap(),
        "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7"
    );
}

#[test]
fn hmac_sha256_matches_rfc4231_case_2() {
    // RFC 4231 test case 2: the ASCII key "Jefe".
    assert_eq!(
        hmac_sha256("Jefe", "what do ya want for nothing?").unwrap(),
        "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843"
    );
}
