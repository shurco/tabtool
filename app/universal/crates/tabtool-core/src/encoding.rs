//! The 18 file encodings offered in the format bar.

use std::borrow::Cow;

use crate::CsvError;

pub(crate) const UTF8_BOM: [u8; 3] = [0xEF, 0xBB, 0xBF];
const UTF16_BE_BOM: [u8; 2] = [0xFE, 0xFF];
const UTF16_LE_BOM: [u8; 2] = [0xFF, 0xFE];
const UTF32_BE_BOM: [u8; 4] = [0x00, 0x00, 0xFE, 0xFF];
const UTF32_LE_BOM: [u8; 4] = [0xFF, 0xFE, 0x00, 0x00];

/// Bytes Windows-1252 leaves undefined, which Foundation rejects.
///
/// `encoding_rs` follows WHATWG and maps them to C1 controls instead, so they
/// are rejected here to keep decoding identical to Apple's Foundation.
const WINDOWS_1252_UNDEFINED: [u8; 5] = [0x81, 0x8D, 0x8F, 0x90, 0x9D];

/// An encoding offered in the format bar.
///
/// [`Encoding::ALL`] lists the encodings in display order with the exact names
/// of Apple's `SupportedEncodings.all`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Encoding {
    /// Unicode (UTF-8).
    Utf8,
    /// Western (Mac OS Roman).
    MacRoman,
    /// Western (Windows Latin 1).
    Windows1252,
    /// Chinese (GBK).
    Gbk,
    /// Central European (ISO Latin 2).
    IsoLatin2,
    /// Central European (Windows Latin 2).
    Windows1250,
    /// Cyrillic (Windows).
    Windows1251,
    /// Greek (Windows).
    Windows1253,
    /// Turkish (Windows).
    Windows1254,
    /// Japanese (EUC).
    EucJp,
    /// Japanese (Shift_JIS).
    ShiftJis,
    /// Japanese (ISO 2022-JP).
    Iso2022Jp,
    /// Unicode (UTF-16); a byte order mark picks the endianness, otherwise
    /// big-endian is assumed.
    Utf16,
    /// Unicode (UTF-16, Big Endian).
    Utf16Be,
    /// Unicode (UTF-16, Little Endian).
    Utf16Le,
    /// Unicode (UTF-32); a byte order mark picks the endianness, otherwise
    /// big-endian is assumed.
    Utf32,
    /// Unicode (UTF-32, Big Endian).
    Utf32Be,
    /// Unicode (UTF-32, Little Endian).
    Utf32Le,
}

impl Encoding {
    /// All supported encodings in display order.
    pub const ALL: [Self; 18] = [
        Self::Utf8,
        Self::MacRoman,
        Self::Windows1252,
        Self::Gbk,
        Self::IsoLatin2,
        Self::Windows1250,
        Self::Windows1251,
        Self::Windows1253,
        Self::Windows1254,
        Self::EucJp,
        Self::ShiftJis,
        Self::Iso2022Jp,
        Self::Utf16,
        Self::Utf16Be,
        Self::Utf16Le,
        Self::Utf32,
        Self::Utf32Be,
        Self::Utf32Le,
    ];

    /// Stable display-order identifier, from 0 for UTF-8 to 17 for UTF-32 LE.
    pub fn id(self) -> u16 {
        Self::ALL
            .iter()
            .position(|&encoding| encoding == self)
            .expect("every encoding is listed in ALL") as u16
    }

    /// The encoding with the given display-order identifier.
    pub fn from_id(id: u16) -> Option<Self> {
        Self::ALL.get(usize::from(id)).copied()
    }

    /// The exact name shown in the format bar.
    pub const fn display_name(self) -> &'static str {
        match self {
            Self::Utf8 => "Unicode (UTF-8)",
            Self::MacRoman => "Western (Mac OS Roman)",
            Self::Windows1252 => "Western (Windows Latin 1)",
            Self::Gbk => "Chinese (GBK)",
            Self::IsoLatin2 => "Central European (ISO Latin 2)",
            Self::Windows1250 => "Central European (Windows Latin 2)",
            Self::Windows1251 => "Cyrillic (Windows)",
            Self::Windows1253 => "Greek (Windows)",
            Self::Windows1254 => "Turkish (Windows)",
            Self::EucJp => "Japanese (EUC)",
            Self::ShiftJis => "Japanese (Shift_JIS)",
            Self::Iso2022Jp => "Japanese (ISO 2022-JP)",
            Self::Utf16 => "Unicode (UTF-16)",
            Self::Utf16Be => "Unicode (UTF-16, Big Endian)",
            Self::Utf16Le => "Unicode (UTF-16, Little Endian)",
            Self::Utf32 => "Unicode (UTF-32)",
            Self::Utf32Be => "Unicode (UTF-32, Big Endian)",
            Self::Utf32Le => "Unicode (UTF-32, Little Endian)",
        }
    }

    /// Decodes bytes, rejecting input that is invalid for this encoding.
    ///
    /// UTF-8 strips a leading byte order mark. The generic UTF-16 and UTF-32
    /// variants honour a byte order mark and otherwise assume big-endian;
    /// the explicit endian variants never consume a byte order mark. Like
    /// Foundation, an incomplete trailing code unit is ignored.
    pub fn decode(self, bytes: &[u8]) -> Result<Cow<'_, str>, CsvError> {
        match self {
            Self::Utf8 => decode_with(encoding_rs::UTF_8, strip_utf8_bom(bytes)),
            Self::MacRoman => decode_with(encoding_rs::MACINTOSH, bytes),
            Self::Windows1252 => decode_windows_1252(bytes),
            Self::Gbk => decode_with(encoding_rs::GBK, bytes),
            Self::IsoLatin2 => decode_with(encoding_rs::ISO_8859_2, bytes),
            Self::Windows1250 => decode_with(encoding_rs::WINDOWS_1250, bytes),
            Self::Windows1251 => decode_with(encoding_rs::WINDOWS_1251, bytes),
            Self::Windows1253 => decode_with(encoding_rs::WINDOWS_1253, bytes),
            Self::Windows1254 => decode_with(encoding_rs::WINDOWS_1254, bytes),
            Self::EucJp => decode_with(encoding_rs::EUC_JP, bytes),
            Self::ShiftJis => decode_with(encoding_rs::SHIFT_JIS, bytes),
            Self::Iso2022Jp => decode_with(encoding_rs::ISO_2022_JP, bytes),
            Self::Utf16 => decode_utf16_bom(bytes).map(Cow::Owned),
            Self::Utf16Be => decode_utf16(bytes, ByteOrder::Big).map(Cow::Owned),
            Self::Utf16Le => decode_utf16(bytes, ByteOrder::Little).map(Cow::Owned),
            Self::Utf32 => decode_utf32_bom(bytes).map(Cow::Owned),
            Self::Utf32Be => decode_utf32(bytes, ByteOrder::Big).map(Cow::Owned),
            Self::Utf32Le => decode_utf32(bytes, ByteOrder::Little).map(Cow::Owned),
        }
    }

    /// Encodes text, rejecting text this encoding cannot represent.
    ///
    /// The generic UTF-16 and UTF-32 variants write a little-endian byte order
    /// mark; the explicit endian variants never write one.
    pub fn encode(self, text: &str) -> Result<Vec<u8>, CsvError> {
        match self {
            Self::Utf8 => encode_with(encoding_rs::UTF_8, text),
            Self::MacRoman => encode_with(encoding_rs::MACINTOSH, text),
            Self::Windows1252 => encode_with(encoding_rs::WINDOWS_1252, text),
            Self::Gbk => encode_with(encoding_rs::GBK, text),
            Self::IsoLatin2 => encode_with(encoding_rs::ISO_8859_2, text),
            Self::Windows1250 => encode_with(encoding_rs::WINDOWS_1250, text),
            Self::Windows1251 => encode_with(encoding_rs::WINDOWS_1251, text),
            Self::Windows1253 => encode_with(encoding_rs::WINDOWS_1253, text),
            Self::Windows1254 => encode_with(encoding_rs::WINDOWS_1254, text),
            Self::EucJp => encode_with(encoding_rs::EUC_JP, text),
            Self::ShiftJis => encode_with(encoding_rs::SHIFT_JIS, text),
            Self::Iso2022Jp => encode_with(encoding_rs::ISO_2022_JP, text),
            Self::Utf16 => Ok(encode_utf16_bom(text)),
            Self::Utf16Be => Ok(encode_utf16(text, ByteOrder::Big)),
            Self::Utf16Le => Ok(encode_utf16(text, ByteOrder::Little)),
            Self::Utf32 => Ok(encode_utf32_bom(text)),
            Self::Utf32Be => Ok(encode_utf32(text, ByteOrder::Big)),
            Self::Utf32Le => Ok(encode_utf32(text, ByteOrder::Little)),
        }
    }
}

/// Byte order of a UTF-16 or UTF-32 code unit.
#[derive(Clone, Copy)]
enum ByteOrder {
    Big,
    Little,
}

fn strip_utf8_bom(bytes: &[u8]) -> &[u8] {
    bytes.strip_prefix(UTF8_BOM.as_slice()).unwrap_or(bytes)
}

fn decode_with<'a>(
    encoding: &'static encoding_rs::Encoding,
    bytes: &'a [u8],
) -> Result<Cow<'a, str>, CsvError> {
    encoding
        .decode_without_bom_handling_and_without_replacement(bytes)
        .ok_or(CsvError::CannotDecodeData)
}

/// Decodes Windows-1252, rejecting the five bytes the code page leaves open.
fn decode_windows_1252(bytes: &[u8]) -> Result<Cow<'_, str>, CsvError> {
    if bytes
        .iter()
        .any(|byte| WINDOWS_1252_UNDEFINED.contains(byte))
    {
        return Err(CsvError::CannotDecodeData);
    }
    decode_with(encoding_rs::WINDOWS_1252, bytes)
}

fn decode_utf16_bom(bytes: &[u8]) -> Result<String, CsvError> {
    if let Some(rest) = bytes.strip_prefix(UTF16_BE_BOM.as_slice()) {
        decode_utf16(rest, ByteOrder::Big)
    } else if let Some(rest) = bytes.strip_prefix(UTF16_LE_BOM.as_slice()) {
        decode_utf16(rest, ByteOrder::Little)
    } else {
        decode_utf16(bytes, ByteOrder::Big)
    }
}

fn decode_utf16(bytes: &[u8], order: ByteOrder) -> Result<String, CsvError> {
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

fn decode_utf32_bom(bytes: &[u8]) -> Result<String, CsvError> {
    if let Some(rest) = bytes.strip_prefix(UTF32_BE_BOM.as_slice()) {
        decode_utf32(rest, ByteOrder::Big)
    } else if let Some(rest) = bytes.strip_prefix(UTF32_LE_BOM.as_slice()) {
        decode_utf32(rest, ByteOrder::Little)
    } else {
        decode_utf32(bytes, ByteOrder::Big)
    }
}

fn decode_utf32(bytes: &[u8], order: ByteOrder) -> Result<String, CsvError> {
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

fn encode_with(encoding: &'static encoding_rs::Encoding, text: &str) -> Result<Vec<u8>, CsvError> {
    let (bytes, _, had_errors) = encoding.encode(text);
    if had_errors {
        Err(CsvError::UnencodableText)
    } else {
        Ok(bytes.into_owned())
    }
}

fn encode_utf16(text: &str, order: ByteOrder) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(text.len() * 2);
    push_utf16(&mut bytes, text, order);
    bytes
}

fn encode_utf16_bom(text: &str) -> Vec<u8> {
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

fn encode_utf32(text: &str, order: ByteOrder) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(text.len() * 4);
    push_utf32(&mut bytes, text, order);
    bytes
}

fn encode_utf32_bom(text: &str) -> Vec<u8> {
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
