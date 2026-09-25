//! Length-prefixed serialization of a [`Bundle`] to and from the raw payload.
//!
//! The payload is deliberately hand-rolled rather than serde-based: blob data is
//! arbitrary binary, and length prefixes keep it exact without a base64 pass.
//! Every read is bounds-checked, so a malformed payload is a typed error.

use vr_core::manifest::Manifest;

use crate::bundle::{Bundle, BundleEntry, EntryKind};
use crate::error::PackError;

/// Payload schema version, independent of the footer's file-format version.
const CODEC_VERSION: u8 = 1;

pub(crate) fn encode(bundle: &Bundle) -> Result<Vec<u8>, PackError> {
    let manifest = bundle.manifest.to_toml()?;
    let count = u32::try_from(bundle.entries().len())
        .map_err(|_| PackError::Malformed("too many entries".to_owned()))?;

    let mut out = Vec::new();
    out.push(CODEC_VERSION);
    put_str(&mut out, &manifest)?;
    out.extend_from_slice(&count.to_le_bytes());
    for entry in bundle.entries() {
        out.push(entry.kind.as_u8());
        put_str(&mut out, &entry.name)?;
        put_blob(&mut out, &entry.data)?;
    }
    Ok(out)
}

pub(crate) fn decode(data: &[u8]) -> Result<Bundle, PackError> {
    let mut cursor = Cursor::new(data);
    let version = cursor.u8()?;
    if version != CODEC_VERSION {
        return Err(PackError::UnsupportedVersion { found: version });
    }

    let manifest = Manifest::from_toml(&cursor.string()?)?;
    let count = cursor.u32()?;
    // Cap the pre-allocation by what is actually left so a corrupt count cannot
    // request a huge buffer before the first read fails.
    let mut entries = Vec::with_capacity((count as usize).min(cursor.remaining() / 8));
    for _ in 0..count {
        let kind = EntryKind::from_u8(cursor.u8()?)?;
        let name = cursor.string()?;
        let data = cursor.blob()?.to_vec();
        entries.push(BundleEntry { kind, name, data });
    }

    if cursor.remaining() != 0 {
        return Err(PackError::Malformed(
            "trailing bytes after the last entry".to_owned(),
        ));
    }

    Ok(Bundle { manifest, entries })
}

fn put_str(out: &mut Vec<u8>, value: &str) -> Result<(), PackError> {
    let len = u32::try_from(value.len())
        .map_err(|_| PackError::Malformed("string too long".to_owned()))?;
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(value.as_bytes());
    Ok(())
}

fn put_blob(out: &mut Vec<u8>, value: &[u8]) -> Result<(), PackError> {
    let len =
        u64::try_from(value.len()).map_err(|_| PackError::Malformed("blob too long".to_owned()))?;
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(value);
    Ok(())
}

/// A bounds-checked forward reader; indexing only happens after a length check.
struct Cursor<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    fn remaining(&self) -> usize {
        self.data.len().saturating_sub(self.pos)
    }

    fn take(&mut self, len: usize) -> Result<&'a [u8], PackError> {
        let end = self
            .pos
            .checked_add(len)
            .ok_or_else(|| PackError::Malformed("length overflow".to_owned()))?;
        if end > self.data.len() {
            return Err(PackError::Malformed("payload is truncated".to_owned()));
        }
        let slice = &self.data[self.pos..end];
        self.pos = end;
        Ok(slice)
    }

    fn u8(&mut self) -> Result<u8, PackError> {
        let bytes = self.take(1)?;
        Ok(bytes[0])
    }

    fn u32(&mut self) -> Result<u32, PackError> {
        let bytes = self.take(4)?;
        Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    fn u64(&mut self) -> Result<u64, PackError> {
        let bytes = self.take(8)?;
        Ok(u64::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]))
    }

    fn string(&mut self) -> Result<String, PackError> {
        let len = usize::try_from(self.u32()?)
            .map_err(|_| PackError::Malformed("string length overflow".to_owned()))?;
        let bytes = self.take(len)?;
        String::from_utf8(bytes.to_vec())
            .map_err(|_| PackError::Malformed("string is not valid UTF-8".to_owned()))
    }

    fn blob(&mut self) -> Result<&'a [u8], PackError> {
        let len = usize::try_from(self.u64()?)
            .map_err(|_| PackError::Malformed("blob length overflow".to_owned()))?;
        self.take(len)
    }
}
