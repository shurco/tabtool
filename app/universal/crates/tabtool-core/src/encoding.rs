//! The 18 file encodings offered in the format bar.

mod unicode;

use std::borrow::Cow;

use unicode::{
    ByteOrder, decode_utf16, decode_utf16_bom, decode_utf32, decode_utf32_bom, encode_utf16,
    encode_utf16_bom, encode_utf32, encode_utf32_bom,
};

use crate::CsvError;

pub(crate) const UTF8_BOM: [u8; 3] = [0xEF, 0xBB, 0xBF];

// Bytes each Windows code page leaves undefined. Foundation rejects them;
// `encoding_rs` follows WHATWG and maps the C1 slots to control characters, so
// they are rejected here to keep decoding identical to Apple's Foundation.
const WINDOWS_1250_UNDEFINED: [u8; 5] = [0x81, 0x83, 0x88, 0x90, 0x98];
const WINDOWS_1251_UNDEFINED: [u8; 1] = [0x98];
const WINDOWS_1252_UNDEFINED: [u8; 5] = [0x81, 0x8D, 0x8F, 0x90, 0x9D];
const WINDOWS_1253_UNDEFINED: [u8; 17] = [
    0x81, 0x88, 0x8A, 0x8C, 0x8D, 0x8E, 0x8F, 0x90, 0x98, 0x9A, 0x9C, 0x9D, 0x9E, 0x9F, 0xAA, 0xD2,
    0xFF,
];
const WINDOWS_1254_UNDEFINED: [u8; 7] = [0x81, 0x8D, 0x8E, 0x8F, 0x90, 0x9D, 0x9E];

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
    ///
    /// Foundation's GBK converter is GB18030-2000, so this maps to
    /// `encoding_rs`'s GB18030 codec rather than its WHATWG GBK codec. The
    /// format bar keeps Apple's display name.
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
    /// Foundation, a trailing incomplete code unit in UTF-16 or UTF-32 input is
    /// ignored instead of raising [`CsvError::CannotDecodeData`]: the tail is
    /// dropped and the preceding scalars are returned.
    ///
    /// The legacy single-byte code pages reject the bytes Foundation leaves
    /// undefined, and Shift_JIS rejects a standalone `0x80` that WHATWG would
    /// decode as U+0080.
    pub fn decode(self, bytes: &[u8]) -> Result<Cow<'_, str>, CsvError> {
        match self {
            Self::Utf8 => decode_with(encoding_rs::UTF_8, strip_utf8_bom(bytes)),
            Self::MacRoman => decode_with(encoding_rs::MACINTOSH, bytes),
            Self::Windows1252 => {
                decode_legacy(encoding_rs::WINDOWS_1252, bytes, &WINDOWS_1252_UNDEFINED)
            }
            Self::Gbk => decode_with(encoding_rs::GB18030, bytes),
            Self::IsoLatin2 => decode_with(encoding_rs::ISO_8859_2, bytes),
            Self::Windows1250 => {
                decode_legacy(encoding_rs::WINDOWS_1250, bytes, &WINDOWS_1250_UNDEFINED)
            }
            Self::Windows1251 => {
                decode_legacy(encoding_rs::WINDOWS_1251, bytes, &WINDOWS_1251_UNDEFINED)
            }
            Self::Windows1253 => {
                decode_legacy(encoding_rs::WINDOWS_1253, bytes, &WINDOWS_1253_UNDEFINED)
            }
            Self::Windows1254 => {
                decode_legacy(encoding_rs::WINDOWS_1254, bytes, &WINDOWS_1254_UNDEFINED)
            }
            Self::EucJp => decode_with(encoding_rs::EUC_JP, bytes),
            Self::ShiftJis => decode_shift_jis(bytes),
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
    /// mark; the explicit endian variants never write one. Legacy code pages
    /// reject C1 controls (U+0080..U+009F) because Foundation cannot encode
    /// them; GBK (GB18030) is the exception and writes them as four-byte
    /// sequences.
    pub fn encode(self, text: &str) -> Result<Vec<u8>, CsvError> {
        match self {
            Self::Utf8 => encode_with(encoding_rs::UTF_8, text),
            Self::MacRoman => encode_legacy(encoding_rs::MACINTOSH, text),
            Self::Windows1252 => encode_legacy(encoding_rs::WINDOWS_1252, text),
            Self::Gbk => encode_gbk(text),
            Self::IsoLatin2 => encode_legacy(encoding_rs::ISO_8859_2, text),
            Self::Windows1250 => encode_legacy(encoding_rs::WINDOWS_1250, text),
            Self::Windows1251 => encode_legacy(encoding_rs::WINDOWS_1251, text),
            Self::Windows1253 => encode_legacy(encoding_rs::WINDOWS_1253, text),
            Self::Windows1254 => encode_legacy(encoding_rs::WINDOWS_1254, text),
            Self::EucJp => encode_legacy(encoding_rs::EUC_JP, text),
            Self::ShiftJis => encode_legacy(encoding_rs::SHIFT_JIS, text),
            Self::Iso2022Jp => encode_legacy(encoding_rs::ISO_2022_JP, text),
            Self::Utf16 => Ok(encode_utf16_bom(text)),
            Self::Utf16Be => Ok(encode_utf16(text, ByteOrder::Big)),
            Self::Utf16Le => Ok(encode_utf16(text, ByteOrder::Little)),
            Self::Utf32 => Ok(encode_utf32_bom(text)),
            Self::Utf32Be => Ok(encode_utf32(text, ByteOrder::Big)),
            Self::Utf32Le => Ok(encode_utf32(text, ByteOrder::Little)),
        }
    }
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

/// Decodes a legacy code page, rejecting the bytes Foundation leaves undefined.
fn decode_legacy<'a>(
    encoding: &'static encoding_rs::Encoding,
    bytes: &'a [u8],
    undefined: &[u8],
) -> Result<Cow<'a, str>, CsvError> {
    if bytes.iter().any(|byte| undefined.contains(byte)) {
        return Err(CsvError::CannotDecodeData);
    }
    decode_with(encoding, bytes)
}

/// Decodes Shift_JIS, rejecting a standalone `0x80`.
///
/// `encoding_rs` maps byte `0x80` to U+0080 while Foundation rejects it. The
/// byte can also appear as the trail of a valid pair (`0x81 0x80` is `÷`), so
/// the decoded text is checked instead of the raw bytes.
fn decode_shift_jis(bytes: &[u8]) -> Result<Cow<'_, str>, CsvError> {
    let text = decode_with(encoding_rs::SHIFT_JIS, bytes)?;
    if text.contains('\u{0080}') {
        Err(CsvError::CannotDecodeData)
    } else {
        Ok(text)
    }
}

fn encode_with(encoding: &'static encoding_rs::Encoding, text: &str) -> Result<Vec<u8>, CsvError> {
    let (bytes, _, had_errors) = encoding.encode(text);
    if had_errors {
        Err(CsvError::UnencodableText)
    } else {
        Ok(bytes.into_owned())
    }
}

/// Encodes a legacy code page, rejecting C1 controls like Foundation does.
///
/// `encoding_rs` maps U+0080..U+009F to their single bytes for several code
/// pages (or rejects them for others); Foundation refuses all of them, so the
/// guard is applied uniformly to every legacy code page except GBK.
fn encode_legacy(
    encoding: &'static encoding_rs::Encoding,
    text: &str,
) -> Result<Vec<u8>, CsvError> {
    if text
        .chars()
        .any(|character| matches!(character, '\u{0080}'..='\u{009F}'))
    {
        return Err(CsvError::UnencodableText);
    }
    encode_with(encoding, text)
}

/// Encodes GBK with Foundation's GB18030-2000 converter.
///
/// Foundation treats a leading U+FEFF as a signature and drops it before
/// encoding; a U+FEFF anywhere else is encoded as a normal character.
fn encode_gbk(text: &str) -> Result<Vec<u8>, CsvError> {
    let text = text.strip_prefix('\u{FEFF}').unwrap_or(text);
    encode_with(encoding_rs::GB18030, text)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Encodings whose C1 controls Foundation refuses to encode.
    const LEGACY_C1_REJECTING: [Encoding; 10] = [
        Encoding::MacRoman,
        Encoding::IsoLatin2,
        Encoding::Windows1250,
        Encoding::Windows1251,
        Encoding::Windows1252,
        Encoding::Windows1253,
        Encoding::Windows1254,
        Encoding::EucJp,
        Encoding::ShiftJis,
        Encoding::Iso2022Jp,
    ];

    #[test]
    fn legacy_encoders_reject_every_c1_control() {
        for encoding in LEGACY_C1_REJECTING {
            for character in ['\u{0080}', '\u{0085}', '\u{009F}'] {
                assert_eq!(
                    encoding.encode(&character.to_string()),
                    Err(CsvError::UnencodableText),
                    "{encoding:?} {character:?}"
                );
            }
            assert_eq!(
                encoding.encode("a\u{0080}b"),
                Err(CsvError::UnencodableText),
                "{encoding:?} inside text"
            );
        }
    }

    #[test]
    fn windows_code_pages_reject_foundation_undefined_bytes() {
        let cases: [(Encoding, &[u8]); 5] = [
            (Encoding::Windows1250, &WINDOWS_1250_UNDEFINED),
            (Encoding::Windows1251, &WINDOWS_1251_UNDEFINED),
            (Encoding::Windows1252, &WINDOWS_1252_UNDEFINED),
            (Encoding::Windows1253, &WINDOWS_1253_UNDEFINED),
            (Encoding::Windows1254, &WINDOWS_1254_UNDEFINED),
        ];
        for (encoding, undefined) in cases {
            for &byte in undefined {
                assert_eq!(
                    encoding.decode(&[b'a', byte, b'b']),
                    Err(CsvError::CannotDecodeData),
                    "{encoding:?} {byte:#04x}"
                );
            }
        }
    }

    #[test]
    fn shift_jis_rejects_a_standalone_80_but_accepts_it_as_a_trail_byte() {
        assert_eq!(
            Encoding::ShiftJis.decode(&[0x80]),
            Err(CsvError::CannotDecodeData)
        );
        // Foundation decodes 0x81 0x80 as U+00F7 (`÷`).
        assert_eq!(Encoding::ShiftJis.decode(&[0x81, 0x80]).unwrap(), "÷");
    }

    #[test]
    fn gbk_matches_foundations_gb18030_converter() {
        assert_eq!(Encoding::Gbk.encode("€").unwrap(), [0xA2, 0xE3]);
        assert_eq!(
            Encoding::Gbk.encode("\u{00A0}").unwrap(),
            [0x81, 0x30, 0x84, 0x32]
        );
        assert_eq!(
            Encoding::Gbk.encode("\u{0080}").unwrap(),
            [0x81, 0x30, 0x81, 0x30]
        );
        assert_eq!(
            Encoding::Gbk.encode("😀").unwrap(),
            [0x94, 0x39, 0xFC, 0x36]
        );
        assert_eq!(
            Encoding::Gbk.encode("日本").unwrap(),
            [0xC8, 0xD5, 0xB1, 0xBE]
        );
        // A leading U+FEFF is treated as a signature and dropped; one in the
        // middle is encoded normally.
        assert_eq!(Encoding::Gbk.encode("\u{FEFF}").unwrap(), Vec::<u8>::new());
        assert_eq!(
            Encoding::Gbk.encode("A\u{FEFF}B").unwrap(),
            [0x41, 0x84, 0x31, 0x95, 0x33, 0x42]
        );
        // Foundation does not strip a GBK byte order mark on decode; the
        // UTF-8 BOM prefix is just GBK bytes to this decoder.
        assert_eq!(
            Encoding::Gbk.decode(&[0xEF, 0xBB, 0xBF, 0x41]).unwrap(),
            "\u{9518}\u{7DFC}"
        );
        assert_eq!(
            Encoding::Gbk.decode(&[0xEF, 0xBB, 0xBF]),
            Err(CsvError::CannotDecodeData)
        );
    }

    #[test]
    fn utf16_and_utf32_drop_a_trailing_incomplete_unit() {
        for encoding in [Encoding::Utf16, Encoding::Utf16Be] {
            assert_eq!(encoding.decode(&[0x00, 0x61, 0x62]).unwrap(), "a");
        }
        assert_eq!(Encoding::Utf16Le.decode(&[0x61, 0x00, 0x62]).unwrap(), "a");
        for encoding in [Encoding::Utf32, Encoding::Utf32Be] {
            assert_eq!(
                encoding.decode(&[0x00, 0x00, 0x00, 0x61, 0x62]).unwrap(),
                "a"
            );
        }
        assert_eq!(
            Encoding::Utf32Le
                .decode(&[0x61, 0x00, 0x00, 0x00, 0x62])
                .unwrap(),
            "a"
        );
    }
}
