//! Message digests and HMAC, rendered as lower-case hexadecimal.
//!
//! The RustCrypto crates are used rather than hand-rolled algorithms: `md-5`,
//! `sha1` and `sha2` share the `digest::Digest` trait so the three one-shot
//! digests collapse to the same call, and `hmac` supplies a constant-time MAC.
//! Every function takes UTF-8 text because Dyon has no byte-string type.
//!
//! These are exposed for compatibility with existing data (file checksums,
//! legacy signatures). MD5 and SHA-1 are broken for collision resistance and
//! must not be chosen for new security work; that is why they carry a warning
//! here but remain available.

use hmac::{Hmac, Mac};
use md5::Md5;
use sha1::Sha1;
use sha2::{Digest, Sha256};

use crate::error::HashError;

/// Lower-case hex MD5 of `text`.
pub fn md5(text: &str) -> String {
    hex::encode(Md5::digest(text.as_bytes()))
}

/// Lower-case hex SHA-1 of `text`.
pub fn sha1(text: &str) -> String {
    hex::encode(Sha1::digest(text.as_bytes()))
}

/// Lower-case hex SHA-256 of `text`.
pub fn sha256(text: &str) -> String {
    hex::encode(Sha256::digest(text.as_bytes()))
}

/// Lower-case hex HMAC-SHA256 of `text` under `key`.
pub fn hmac_sha256(key: &str, text: &str) -> Result<String, HashError> {
    let mut mac =
        Hmac::<Sha256>::new_from_slice(key.as_bytes()).map_err(|_| HashError::InvalidKey)?;
    mac.update(text.as_bytes());
    Ok(hex::encode(mac.finalize().into_bytes()))
}

#[cfg(test)]
mod tests;
