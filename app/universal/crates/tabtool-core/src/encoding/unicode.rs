//! UTF-16 and UTF-32 decoding and encoding, with and without a byte order mark.

use crate::CsvError;

const UTF16_BE_BOM: [u8; 2] = [0xFE, 0xFF];
const UTF16_LE_BOM: [u8; 2] = [0xFF, 0xFE];
const UTF32_BE_BOM: [u8; 4] = [0x00, 0x00, 0xFE, 0xFF];
const UTF32_LE_BOM: [u8; 4] = [0xFF, 0xFE, 0x00, 0x00];

/// Byte order of a UTF-16 or UTF-32 code unit.
#[derive(Clone, Copy)]
pub(super) enum ByteOrder {
    Big,
    Little,
}

pub(super) fn decode_utf16_bom(bytes: &[u8]) -> Result<String, CsvError> {
    if let Some(rest) = bytes.strip_prefix(UTF16_BE_BOM.as_slice()) {
        decode_utf16(rest, ByteOrder::Big)
    } else if let Some(rest) = bytes.strip_prefix(UTF16_LE_BOM.as_slice()) {
        decode_utf16(rest, ByteOrder::Little)
    } else {
        decode_utf16(bytes, ByteOrder::Big)
    }
}

pub(super) fn decode_utf16(bytes: &[u8], order: ByteOrder) -> Result<String, CsvError> {
    // Foundation ignores a trailing incomplete code unit.
    let (pairs, _remainder) = bytes.as_chunks::<2>();
    let units = pairs.iter().map(|pair| match order {
        ByteOrder::Big => u16::from_be_bytes(*pair),
        ByteOrder::Little => u16::from_le_bytes(*pair),
    });

    char::decode_utf16(units)
        .collect::<Result<String, _>>()
        .map_err(|_| CsvError::CannotDecodeData)
}

pub(super) fn decode_utf32_bom(bytes: &[u8]) -> Result<String, CsvError> {
    if let Some(rest) = bytes.strip_prefix(UTF32_BE_BOM.as_slice()) {
        decode_utf32(rest, ByteOrder::Big)
    } else if let Some(rest) = bytes.strip_prefix(UTF32_LE_BOM.as_slice()) {
        decode_utf32(rest, ByteOrder::Little)
    } else {
        decode_utf32(bytes, ByteOrder::Big)
    }
}

pub(super) fn decode_utf32(bytes: &[u8], order: ByteOrder) -> Result<String, CsvError> {
    // Foundation ignores trailing bytes that do not form a whole code unit.
    let mut text = String::with_capacity(bytes.len() / 4);
    let (chunks, _remainder) = bytes.as_chunks::<4>();
    for chunk in chunks {
        let value = match order {
            ByteOrder::Big => u32::from_be_bytes(*chunk),
            ByteOrder::Little => u32::from_le_bytes(*chunk),
        };
        text.push(char::from_u32(value).ok_or(CsvError::CannotDecodeData)?);
    }
    Ok(text)
}

pub(super) fn encode_utf16(text: &str, order: ByteOrder) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(text.len() * 2);
    push_utf16(&mut bytes, text, order);
    bytes
}

pub(super) fn encode_utf16_bom(text: &str) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(UTF16_LE_BOM.len() + text.len() * 2);
    bytes.extend_from_slice(&UTF16_LE_BOM);
    push_utf16(&mut bytes, text, ByteOrder::Little);
    bytes
}

fn push_utf16(bytes: &mut Vec<u8>, text: &str, order: ByteOrder) {
    for unit in text.encode_utf16() {
        let encoded = match order {
            ByteOrder::Big => unit.to_be_bytes(),
            ByteOrder::Little => unit.to_le_bytes(),
        };
        bytes.extend_from_slice(&encoded);
    }
}

pub(super) fn encode_utf32(text: &str, order: ByteOrder) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(text.len() * 4);
    push_utf32(&mut bytes, text, order);
    bytes
}

pub(super) fn encode_utf32_bom(text: &str) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(UTF32_LE_BOM.len() + text.len() * 4);
    bytes.extend_from_slice(&UTF32_LE_BOM);
    push_utf32(&mut bytes, text, ByteOrder::Little);
    bytes
}

fn push_utf32(bytes: &mut Vec<u8>, text: &str, order: ByteOrder) {
    for character in text.chars() {
        let value = u32::from(character);
        let encoded = match order {
            ByteOrder::Big => value.to_be_bytes(),
            ByteOrder::Little => value.to_le_bytes(),
        };
        bytes.extend_from_slice(&encoded);
    }
}
