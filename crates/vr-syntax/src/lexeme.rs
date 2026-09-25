//! Single-lexeme consumers shared by the line scanner.
//!
//! Each function is handed the line bytes and the index where the lexeme is
//! believed to start, and returns the exclusive end (and, for operators, the
//! token class). None of them look at the lexer state: only block comments
//! outlive a line.

use crate::style::TokenKind;

/// Numeric type suffixes that may trail a literal (`1.0f64`, `3u32`).
const NUMBER_SUFFIXES: &[&str] = &[
    "f32", "f64", "i8", "i16", "i32", "i64", "u8", "u16", "u32", "u64",
];

const OPERATORS_3: &[&[u8]] = &[b"<<=", b">>=", b"..="];
const OPERATORS_2: &[&[u8]] = &[
    b"==", b"!=", b"<=", b">=", b"&&", b"||", b"->", b"=>", b"::", b"+=", b"-=", b"*=", b"/=",
    b"%=", b"&=", b"|=", b"^=", b"<<", b">>", b"..",
];

/// Recognises `///` (but not `////`, which Rust and Dyon treat as a plain
/// comment) and `//!`.
pub(crate) fn is_doc_line_comment(bytes: &[u8], index: usize) -> bool {
    let rest = &bytes[index..];
    (rest.starts_with(b"///") && rest.get(3) != Some(&b'/')) || rest.starts_with(b"//!")
}

/// Consumes a string literal, honouring backslash escapes so an escaped quote
/// does not end the run. An unterminated string runs to the end of the line.
pub(crate) fn scan_string(bytes: &[u8], start: usize) -> usize {
    let mut index = start + 1;
    while index < bytes.len() {
        match bytes[index] {
            b'\\' => {
                index += 1;
                if index >= bytes.len() {
                    break;
                }
                if bytes[index] == b'u' && bytes.get(index + 1) == Some(&b'{') {
                    index += 2;
                    while index < bytes.len() && bytes[index] != b'}' {
                        index += 1;
                    }
                    if index < bytes.len() {
                        index += 1;
                    }
                } else {
                    index += char_len_at(bytes, index);
                }
            }
            b'"' => return index + 1,
            _ => index += char_len_at(bytes, index),
        }
    }
    bytes.len()
}

/// Consumes an integer, float, exponent or base-prefixed literal, plus an
/// optional type suffix. The exponent marker `e` is only part of the number
/// when digits actually follow, so `1each` lexes as `1` then `each`.
pub(crate) fn scan_number(bytes: &[u8], start: usize) -> usize {
    let mut index = start;
    if bytes[start] == b'0'
        && matches!(
            bytes.get(start + 1),
            Some(&b'x' | b'X' | b'b' | b'B' | b'o' | b'O')
        )
    {
        index += 2;
        while index < bytes.len() && (bytes[index].is_ascii_alphanumeric() || bytes[index] == b'_')
        {
            index += 1;
        }
        return index;
    }

    index = scan_digits(bytes, index);
    if bytes.get(index) == Some(&b'.') && bytes.get(index + 1).is_some_and(u8::is_ascii_digit) {
        index = scan_digits(bytes, index + 1);
    }
    if matches!(bytes.get(index), Some(&b'e' | b'E')) {
        let mut exponent = index + 1;
        if matches!(bytes.get(exponent), Some(&b'+' | b'-')) {
            exponent += 1;
        }
        if bytes.get(exponent).is_some_and(u8::is_ascii_digit) {
            index = scan_digits(bytes, exponent);
        }
    }

    let suffix_end = scan_ident(bytes, index);
    if suffix_end > index {
        let suffix = std::str::from_utf8(&bytes[index..suffix_end]).unwrap_or("");
        if NUMBER_SUFFIXES.contains(&suffix) {
            index = suffix_end;
        }
    }
    index
}

/// Consumes one operator or punctuation token, preferring the longest match.
/// Bytes that are neither ASCII punctuation nor part of a valid token become a
/// one-character [`TokenKind::Error`] so the scanner always advances.
pub(crate) fn scan_operator(bytes: &[u8], index: usize) -> (usize, TokenKind) {
    let byte = bytes[index];
    if !byte.is_ascii() {
        return (char_len_at(bytes, index), TokenKind::Error);
    }
    if !is_punctuation(byte) {
        return (1, TokenKind::Error);
    }
    let rest = &bytes[index..];
    if OPERATORS_3.iter().any(|op| rest.starts_with(op)) {
        return (3, TokenKind::Operator);
    }
    if OPERATORS_2.iter().any(|op| rest.starts_with(op)) {
        return (2, TokenKind::Operator);
    }
    (1, TokenKind::Operator)
}

fn scan_digits(bytes: &[u8], mut index: usize) -> usize {
    while index < bytes.len() && (bytes[index].is_ascii_digit() || bytes[index] == b'_') {
        index += 1;
    }
    index
}

fn scan_ident(bytes: &[u8], mut index: usize) -> usize {
    if index < bytes.len() && is_ident_start(bytes[index]) {
        index += 1;
        while index < bytes.len() && is_ident_continue(bytes[index]) {
            index += 1;
        }
    }
    index
}

pub(crate) fn is_ident_start(byte: u8) -> bool {
    byte == b'_' || byte.is_ascii_alphabetic()
}

pub(crate) fn is_ident_continue(byte: u8) -> bool {
    byte == b'_' || byte.is_ascii_alphanumeric()
}

fn is_punctuation(byte: u8) -> bool {
    matches!(
        byte,
        b'+' | b'-'
            | b'*'
            | b'/'
            | b'%'
            | b'='
            | b'<'
            | b'>'
            | b'!'
            | b'&'
            | b'|'
            | b'^'
            | b'~'
            | b'?'
            | b':'
            | b';'
            | b','
            | b'.'
            | b'('
            | b')'
            | b'['
            | b']'
            | b'{'
            | b'}'
    )
}

/// Length of the UTF-8 sequence starting at `index`; an invalid lead or stray
/// continuation byte advances one so the scanner cannot stall.
fn char_len_at(bytes: &[u8], index: usize) -> usize {
    match bytes[index] {
        0x00..=0x7F => 1,
        0xC0..=0xDF => 2,
        0xE0..=0xEF => 3,
        0xF0..=0xF7 => 4,
        _ => 1,
    }
}
