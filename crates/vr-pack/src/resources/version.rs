//! Builds a `VS_VERSIONINFO` resource block from the project manifest.
//!
//! `UpdateResource` wants the raw binary here-document, not an `.rc` source, so
//! the block is assembled by hand. The layout and the alignment/`wValueLength`
//! conventions were checked against `rc.exe` output for the same fields: every
//! member is DWORD-aligned, `VS_VERSIONINFO`'s value length is the 52-byte
//! `VS_FIXEDFILEINFO`, and a string `Value`'s length is its UTF-16 code-unit
//! count *including* the terminator.

use crate::error::PackError;

/// The language/codepage pair the `StringTable` key encodes. English (US) with
/// the Unicode codepage is what `rc.exe` emits by default and what the patcher
/// registers the resource under.
pub(crate) const LANG_EN_US: u16 = 0x0409;
const CODEPAGE_UNICODE: u16 = 0x04b0;

/// The fixed and string fields to embed. `file_version`/`product_version` are
/// the normalized `a.b.c.d` forms so the table's text and the binary pair agree.
pub(crate) struct VersionFields<'a> {
    pub file_description: &'a str,
    pub product_name: &'a str,
    pub file_version: String,
    pub product_version: String,
    pub fixed: [u16; 4],
}

/// Parses a manifest version into the four `u16` words `VS_FIXEDFILEINFO`
/// stores. Components after a non-digit suffix are ignored, so `1.2.3-beta`
/// yields `1.2.3.0`; a component that starts with no digit at all is an error.
pub(crate) fn parse_version(value: &str) -> Result<[u16; 4], PackError> {
    let mut words = [0u16; 4];
    let parts: Vec<&str> = value.split('.').collect();
    if parts.len() > 4 {
        return Err(invalid(value, "more than four dot-separated components"));
    }
    for (index, part) in parts.iter().enumerate() {
        words[index] = parse_component(value, part)?;
    }
    Ok(words)
}

fn parse_component(full: &str, part: &str) -> Result<u16, PackError> {
    let digits: String = part.chars().take_while(char::is_ascii_digit).collect();
    if digits.is_empty() {
        return Err(invalid(full, "a component does not start with a digit"));
    }
    digits
        .parse::<u16>()
        .map_err(|_| invalid(full, "a component is larger than 65535"))
}

fn invalid(value: &str, reason: &'static str) -> PackError {
    PackError::InvalidVersion {
        value: value.to_owned(),
        reason,
    }
}

/// Assembles the complete `VS_VERSIONINFO` bytes for `fields`.
pub(crate) fn build(fields: &VersionFields<'_>) -> Vec<u8> {
    let fixed = fixed_file_info(fields.fixed);

    let mut strings = Vec::new();
    push_string(&mut strings, "FileDescription", fields.file_description);
    push_string(&mut strings, "FileVersion", &fields.file_version);
    push_string(&mut strings, "ProductName", fields.product_name);
    push_string(&mut strings, "ProductVersion", &fields.product_version);

    let mut table = Vec::new();
    push_block(&mut table, &table_key(), 1, 0, &[], &strings);

    let mut string_file_info = Vec::new();
    push_block(&mut string_file_info, "StringFileInfo", 1, 0, &[], &table);

    let mut translation = Vec::new();
    let langid = u32::from(LANG_EN_US) | (u32::from(CODEPAGE_UNICODE) << 16);
    push_block(
        &mut translation,
        "Translation",
        0,
        4,
        &langid.to_le_bytes(),
        &[],
    );
    let mut var_file_info = Vec::new();
    push_block(&mut var_file_info, "VarFileInfo", 1, 0, &[], &translation);

    let mut children = string_file_info;
    children.extend_from_slice(&var_file_info);

    let mut out = Vec::new();
    push_block(&mut out, "VS_VERSION_INFO", 0, 52, &fixed, &children);
    out
}

/// The `StringTable` key: language and codepage as four hex digits each.
fn table_key() -> String {
    format!("{LANG_EN_US:04x}{CODEPAGE_UNICODE:04x}")
}

fn fixed_file_info([major, minor, build, revision]: [u16; 4]) -> Vec<u8> {
    let mut out = Vec::with_capacity(52);
    push_u32(&mut out, 0xFEEF_04BD); // VS_FFI_SIGNATURE
    push_u32(&mut out, 0x0001_0000); // VS_FFI_STRUCVERSION
    push_u32(&mut out, pair(major, minor)); // FILEVERSION MS
    push_u32(&mut out, pair(build, revision)); // FILEVERSION LS
    push_u32(&mut out, pair(major, minor)); // PRODUCTVERSION MS
    push_u32(&mut out, pair(build, revision)); // PRODUCTVERSION LS
    push_u32(&mut out, 0x3F); // VS_FFI_FILEFLAGSMASK
    push_u32(&mut out, 0); // file flags
    push_u32(&mut out, 0x0004_0004); // VOS_NT_WINDOWS32
    push_u32(&mut out, 0x0000_0001); // VFT_APP
    push_u32(&mut out, 0); // subtype
    push_u32(&mut out, 0); // file date MS
    push_u32(&mut out, 0); // file date LS
    out
}

fn pair(high: u16, low: u16) -> u32 {
    (u32::from(high) << 16) | u32::from(low)
}

/// Appends a `String` entry whose `Value` is `text` plus a NUL.
fn push_string(out: &mut Vec<u8>, key: &str, text: &str) {
    let mut value = Vec::new();
    for unit in text.encode_utf16().chain(std::iter::once(0)) {
        push_u16(&mut value, unit);
    }
    // UTF-16 code units, terminator included, per the VS_VERSIONINFO spec.
    let value_length = u16::try_from(value.len() / 2).unwrap_or(u16::MAX);
    push_block(out, key, 1, value_length, &value, &[]);
}

/// Appends one version block: a 6-byte header, the key, the value and children,
/// all DWORD-aligned. `wLength` is back-patched once the size is known.
fn push_block(
    out: &mut Vec<u8>,
    key: &str,
    value_type: u16,
    value_length: u16,
    value: &[u8],
    children: &[u8],
) {
    let start = out.len();
    push_u16(out, 0);
    push_u16(out, value_length);
    push_u16(out, value_type);
    for unit in key.encode_utf16().chain(std::iter::once(0)) {
        push_u16(out, unit);
    }
    align4(out);
    out.extend_from_slice(value);
    align4(out);
    out.extend_from_slice(children);
    align4(out);
    let length = u16::try_from(out.len() - start).unwrap_or(u16::MAX);
    out[start..start + 2].copy_from_slice(&length.to_le_bytes());
}

fn align4(out: &mut Vec<u8>) {
    while !out.len().is_multiple_of(4) {
        out.push(0);
    }
}

fn push_u16(out: &mut Vec<u8>, value: u16) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn push_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fields() -> VersionFields<'static> {
        VersionFields {
            file_description: "Demo",
            product_name: "VisualRust Demo",
            file_version: "1.2.3.4".to_owned(),
            product_version: "1.2.3.4".to_owned(),
            fixed: [1, 2, 3, 4],
        }
    }

    /// Walks one block header and returns `(wLength, wValueLength, wType, key)`.
    fn read_header(bytes: &[u8], offset: usize) -> (u16, u16, u16, String) {
        let length = u16::from_le_bytes([bytes[offset], bytes[offset + 1]]);
        let value_length = u16::from_le_bytes([bytes[offset + 2], bytes[offset + 3]]);
        let value_type = u16::from_le_bytes([bytes[offset + 4], bytes[offset + 5]]);
        let mut key = String::new();
        let mut cursor = offset + 6;
        while cursor + 1 < bytes.len() {
            let unit = u16::from_le_bytes([bytes[cursor], bytes[cursor + 1]]);
            cursor += 2;
            if unit == 0 {
                break;
            }
            key.push(char::from_u32(u32::from(unit)).unwrap_or('?'));
        }
        (length, value_length, value_type, key)
    }

    #[test]
    fn version_parses_and_normalizes() {
        assert_eq!(parse_version("1").unwrap_or_default(), [1, 0, 0, 0]);
        assert_eq!(parse_version("1.2").unwrap_or_default(), [1, 2, 0, 0]);
        assert_eq!(parse_version("1.2.3").unwrap_or_default(), [1, 2, 3, 0]);
        assert_eq!(parse_version("1.2.3.4").unwrap_or_default(), [1, 2, 3, 4]);
        assert_eq!(
            parse_version("1.2.3-beta").unwrap_or_default(),
            [1, 2, 3, 0]
        );
        assert!(parse_version("beta").is_err());
        assert!(parse_version("1.2.3.4.5").is_err());
    }

    #[test]
    fn root_block_matches_rc_layout() {
        let bytes = build(&fields());
        let (length, value_length, value_type, key) = read_header(&bytes, 0);
        assert_eq!(usize::from(length), bytes.len(), "wLength spans the block");
        assert_eq!(value_length, 52, "VS_FIXEDFILEINFO is 52 bytes");
        assert_eq!(value_type, 0, "binary root");
        assert_eq!(key, "VS_VERSION_INFO");

        // The key is NUL-terminated, then padded to a DWORD boundary.
        let key_end = 6 + (key.len() + 1) * 2;
        assert_eq!(key_end % 4, 2);
        let signature = u32::from_le_bytes([
            bytes[key_end + 2],
            bytes[key_end + 3],
            bytes[key_end + 4],
            bytes[key_end + 5],
        ]);
        assert_eq!(signature, 0xFEEF_04BD);
    }

    #[test]
    fn string_entry_encodes_value_length_in_words_including_nul() {
        let mut out = Vec::new();
        push_string(&mut out, "ProductName", "VisualRust Demo");
        let (length, value_length, value_type, key) = read_header(&out, 0);
        assert_eq!(key, "ProductName");
        assert_eq!(value_type, 1);
        assert_eq!(value_length, 16, "15 chars + NUL");
        assert_eq!(usize::from(length), out.len());
    }

    #[test]
    fn tree_contains_product_and_version_strings() {
        let bytes = build(&fields());
        let text = String::from_utf16_lossy(
            &bytes
                .as_chunks::<2>()
                .0
                .iter()
                .map(|pair| u16::from_le_bytes(*pair))
                .collect::<Vec<_>>(),
        );
        assert!(text.contains("StringFileInfo"));
        assert!(text.contains("VarFileInfo"));
        assert!(text.contains("040904b0"));
        assert!(text.contains("ProductName"));
        assert!(text.contains("VisualRust Demo"));
        assert!(text.contains("1.2.3.4"));
    }
}
