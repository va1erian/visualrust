//! File framing: compress a payload, append the EOF footer, and read it back.

use std::fs::{File, OpenOptions};
use std::io::{Cursor, Read, Seek, SeekFrom, Write};
use std::path::Path;

use flate2::Compression;
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use sha2::{Digest, Sha256};

use crate::bundle::Bundle;
use crate::codec;
use crate::error::PackError;

/// Identifies a VisualRust bundle footer; "\0" pads it to a fixed width.
pub const MAGIC: [u8; 8] = *b"VRBUNDL\0";

/// Bumped when the framing changes incompatibly; readers reject other values.
const FORMAT_VERSION: u8 = 1;

/// Only deflate exists today; the tag leaves room for a future compressor.
const COMPRESSOR_DEFLATE: u8 = 1;

/// The footer is a fixed size so a reader can seek to the last `FOOTER_LEN`
/// bytes instead of scanning the (unknown, PE-sized) file.
pub const FOOTER_LEN: usize = 68;

/// The parsed trailer of a bundle body.
struct Footer {
    payload_offset: u64,
    payload_len: u64,
    raw_len: u64,
    digest: [u8; 32],
}

impl Bundle {
    /// Serializes this bundle for a file that starts at offset `base`: the
    /// compressed payload followed by the footer.
    fn encode_at(&self, base: u64) -> Result<Vec<u8>, PackError> {
        let raw = codec::encode(self)?;
        let compressed = compress(&raw)?;
        // `base` is where the payload begins, i.e. the current end of file.
        let footer = encode_footer(base, compressed.len(), raw.len(), &compressed);
        let mut out = compressed;
        out.extend_from_slice(&footer);
        Ok(out)
    }

    /// Serializes to a standalone byte buffer (payload offset zero).
    pub fn to_bytes(&self) -> Result<Vec<u8>, PackError> {
        self.encode_at(0)
    }

    /// Overwrites `path` with this bundle.
    pub fn write_to_file(&self, path: impl AsRef<Path>) -> Result<(), PackError> {
        let path = path.as_ref();
        std::fs::write(path, self.encode_at(0)?)?;
        Ok(())
    }

    /// Appends this bundle to an existing file (e.g. a PE stub). The existing
    /// bytes, including the PE header, are left untouched.
    pub fn append_to_file(&self, path: impl AsRef<Path>) -> Result<(), PackError> {
        let path = path.as_ref();
        let mut file = OpenOptions::new().append(true).open(path)?;
        let base = file.metadata()?.len();
        file.write_all(&self.encode_at(base)?)?;
        file.flush()?;
        Ok(())
    }

    /// Reads the bundle appended to `path`.
    pub fn read_from_file(path: impl AsRef<Path>) -> Result<Self, PackError> {
        let mut file = File::open(path.as_ref())?;
        read_reader(&mut file)
    }

    /// Reads a bundle already in memory (used by tests and the runtime loader).
    pub fn read_bytes(data: &[u8]) -> Result<Self, PackError> {
        read_reader(&mut Cursor::new(data))
    }
}

fn compress(raw: &[u8]) -> Result<Vec<u8>, PackError> {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(raw)?;
    Ok(encoder.finish()?)
}

fn decompress(compressed: &[u8], raw_len: u64) -> Result<Vec<u8>, PackError> {
    let mut decoder = ZlibDecoder::new(compressed);
    let mut raw = Vec::new();
    decoder
        .read_to_end(&mut raw)
        .map_err(|err| PackError::Decompress(err.to_string()))?;
    if raw.len() as u64 != raw_len {
        return Err(PackError::Malformed(format!(
            "payload length {} does not match footer's {raw_len}",
            raw.len()
        )));
    }
    Ok(raw)
}

fn encode_footer(
    offset: u64,
    payload_len: usize,
    raw_len: usize,
    payload: &[u8],
) -> [u8; FOOTER_LEN] {
    let mut digest = [0u8; 32];
    digest.copy_from_slice(&Sha256::digest(payload));
    let mut footer = [0u8; FOOTER_LEN];
    footer[0..8].copy_from_slice(&MAGIC);
    footer[8] = FORMAT_VERSION;
    footer[9] = COMPRESSOR_DEFLATE;
    footer[12..20].copy_from_slice(&offset.to_le_bytes());
    footer[20..28].copy_from_slice(&(payload_len as u64).to_le_bytes());
    footer[28..36].copy_from_slice(&(raw_len as u64).to_le_bytes());
    footer[36..68].copy_from_slice(&digest);
    footer
}

fn decode_footer(bytes: &[u8]) -> Result<Footer, PackError> {
    if bytes.len() < FOOTER_LEN {
        return Err(PackError::TooShort);
    }
    if bytes[0..8] != MAGIC {
        return Err(PackError::MissingFooter);
    }
    if bytes[8] != FORMAT_VERSION {
        return Err(PackError::UnsupportedVersion { found: bytes[8] });
    }
    if bytes[9] != COMPRESSOR_DEFLATE {
        return Err(PackError::CorruptFooter {
            reason: "unknown compressor tag",
        });
    }

    let mut digest = [0u8; 32];
    digest.copy_from_slice(&bytes[36..68]);
    Ok(Footer {
        payload_offset: u64::from_le_bytes(read_array(&bytes[12..20])),
        payload_len: u64::from_le_bytes(read_array(&bytes[20..28])),
        raw_len: u64::from_le_bytes(read_array(&bytes[28..36])),
        digest,
    })
}

fn read_array(bytes: &[u8]) -> [u8; 8] {
    let mut out = [0u8; 8];
    out.copy_from_slice(bytes);
    out
}

fn read_reader<R: Read + Seek>(reader: &mut R) -> Result<Bundle, PackError> {
    let end = reader.seek(SeekFrom::End(0))?;
    if end < FOOTER_LEN as u64 {
        return Err(PackError::TooShort);
    }

    reader.seek(SeekFrom::End(-(FOOTER_LEN as i64)))?;
    let mut footer_bytes = [0u8; FOOTER_LEN];
    reader.read_exact(&mut footer_bytes)?;
    let footer = decode_footer(&footer_bytes)?;

    let payload_end = footer
        .payload_offset
        .checked_add(footer.payload_len)
        .ok_or(PackError::Truncated)?;
    if footer.payload_offset >= end || payload_end > end {
        return Err(PackError::Truncated);
    }

    let len = usize::try_from(footer.payload_len).map_err(|_| PackError::Truncated)?;
    reader.seek(SeekFrom::Start(footer.payload_offset))?;
    let mut compressed = vec![0u8; len];
    reader.read_exact(&mut compressed)?;

    let digest = Sha256::digest(&compressed);
    if digest[..] != footer.digest[..] {
        return Err(PackError::HashMismatch);
    }

    codec::decode(&decompress(&compressed, footer.raw_len)?)
}
