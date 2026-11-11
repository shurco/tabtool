//! Ports of the Apple `CSVConfigurationTests`, `CSVErrorTests` and
//! `SupportedEncodingsTests` suites, plus encoding boundary checks.
//!
//! Apple's English error descriptions and recovery suggestions belong to the
//! UI plan, so the error tests assert only the enum cases and their equality.

use tabtool_core::{CsvConfiguration, CsvError, Encoding};

/// The Apple display order and exact display names.
const EXPECTED_NAMES: [&str; 18] = [
    "Unicode (UTF-8)",
    "Western (Mac OS Roman)",
    "Western (Windows Latin 1)",
    "Chinese (GBK)",
    "Central European (ISO Latin 2)",
    "Central European (Windows Latin 2)",
    "Cyrillic (Windows)",
    "Greek (Windows)",
    "Turkish (Windows)",
    "Japanese (EUC)",
    "Japanese (Shift_JIS)",
    "Japanese (ISO 2022-JP)",
    "Unicode (UTF-16)",
    "Unicode (UTF-16, Big Endian)",
    "Unicode (UTF-16, Little Endian)",
    "Unicode (UTF-32)",
    "Unicode (UTF-32, Big Endian)",
    "Unicode (UTF-32, Little Endian)",
];

const ERRORS: [CsvError; 8] = [
    CsvError::CannotDecodeData,
    CsvError::MalformedQuotedField,
    CsvError::QuoteInUnquotedField,
    CsvError::UnencodableText,
    CsvError::AmbiguousOutput,
    CsvError::UnsupportedLargeFileEncoding,
    CsvError::UnsupportedLargeFileDelimiter,
    CsvError::RowNotEditable,
];

#[test]
fn default_configuration_matches_defaults() {
    let configuration = CsvConfiguration::default();

    assert_eq!(configuration.encoding, Encoding::Utf8);
    assert_eq!(configuration.column_separator, ",");
    assert_eq!(configuration.quote_character, "\"");
    assert_eq!(configuration.escape_character, "\"");
    assert_eq!(configuration.decimal_mark, ".");
    assert!(!configuration.first_row_as_header);
}

#[test]
fn configuration_stores_custom_values() {
    let mut configuration = CsvConfiguration {
        encoding: Encoding::Windows1252,
        column_separator: ";".to_string(),
        quote_character: "'".to_string(),
        escape_character: "\\".to_string(),
        decimal_mark: ",".to_string(),
        first_row_as_header: true,
    };

    assert_eq!(configuration.encoding, Encoding::Windows1252);
    assert_eq!(configuration.column_separator, ";");
    assert_eq!(configuration.quote_character, "'");
    assert_eq!(configuration.escape_character, "\\");
    assert_eq!(configuration.decimal_mark, ",");
    assert!(configuration.first_row_as_header);

    configuration.column_separator = "|".to_string();
    assert_eq!(configuration.column_separator, "|");
}

#[test]
fn every_error_case_is_distinct_and_comparable() {
    for (index, error) in ERRORS.iter().enumerate() {
        assert_eq!(error, &ERRORS[index]);
        for (other_index, other) in ERRORS.iter().enumerate() {
            assert_eq!(
                index == other_index,
                error == other,
                "{error:?} vs {other:?}"
            );
        }
    }
}

#[test]
fn supported_encodings_match_display_order() {
    assert_eq!(Encoding::ALL.len(), 18);
    assert_eq!(Encoding::ALL[0].display_name(), "Unicode (UTF-8)");
    assert_eq!(Encoding::ALL[0], Encoding::Utf8);
    assert_eq!(Encoding::ALL[3].display_name(), "Chinese (GBK)");
    assert!(Encoding::ALL.contains(&Encoding::Windows1252));
    assert!(Encoding::ALL.contains(&Encoding::Gbk));
}

#[test]
fn supported_encodings_match_expected_table() {
    assert_eq!(Encoding::ALL.len(), EXPECTED_NAMES.len());

    for (encoding, expected) in Encoding::ALL.iter().zip(EXPECTED_NAMES) {
        assert_eq!(encoding.display_name(), expected);
    }
}

#[test]
fn supported_encoding_identifiers_are_unique() {
    let mut ids: Vec<u16> = Encoding::ALL.iter().map(|encoding| encoding.id()).collect();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), Encoding::ALL.len());
}

#[test]
fn identifiers_follow_display_order_and_round_trip() {
    for (index, encoding) in Encoding::ALL.iter().enumerate() {
        assert_eq!(usize::from(encoding.id()), index);
        assert_eq!(Encoding::from_id(encoding.id()), Some(*encoding));
    }

    assert_eq!(Encoding::from_id(18), None);
    assert_eq!(Encoding::from_id(u16::MAX), None);
}

#[test]
fn utf8_bom_is_stripped_by_decode() {
    assert_eq!(Encoding::Utf8.decode(b"\xEF\xBB\xBFa").unwrap(), "a");
}

#[test]
fn utf8_encode_writes_no_bom() {
    assert_eq!(Encoding::Utf8.encode("a").unwrap(), b"a");
}

#[test]
fn unrepresentable_text_fails() {
    assert_eq!(
        Encoding::Windows1251.encode("😀"),
        Err(CsvError::UnencodableText)
    );
}

#[test]
fn utf16_and_utf32_round_trip_every_scalar_boundary() {
    for encoding in [
        Encoding::Utf16,
        Encoding::Utf16Be,
        Encoding::Utf16Le,
        Encoding::Utf32,
        Encoding::Utf32Be,
        Encoding::Utf32Le,
    ] {
        let text = "a\u{1F600}\u{4E2D}";
        assert_eq!(
            encoding.decode(&encoding.encode(text).unwrap()).unwrap(),
            text
        );
    }
}

#[test]
fn invalid_bytes_are_a_typed_error() {
    assert_eq!(
        Encoding::Utf8.decode(b"\xFF"),
        Err(CsvError::CannotDecodeData)
    );
}

#[test]
fn legacy_decoders_do_not_substitute_replacements() {
    // A replacement character would give `Ok`, so `Err` proves the decoders
    // reject truncated multi-byte sequences instead of substituting U+FFFD.
    assert_eq!(
        Encoding::ShiftJis.decode(b"\x81"),
        Err(CsvError::CannotDecodeData)
    );
    assert_eq!(
        Encoding::EucJp.decode(b"\xA4"),
        Err(CsvError::CannotDecodeData)
    );
    assert_eq!(
        Encoding::Iso2022Jp.decode(b"\x1B"),
        Err(CsvError::CannotDecodeData)
    );
}

#[test]
fn legacy_encodings_use_the_expected_code_pages() {
    assert_eq!(
        Encoding::Windows1251.encode("Привет").unwrap(),
        [0xCF, 0xF0, 0xE8, 0xE2, 0xE5, 0xF2]
    );
    assert_eq!(
        Encoding::ShiftJis.encode("日本語").unwrap(),
        [0x93, 0xFA, 0x96, 0x7B, 0x8C, 0xEA]
    );
    assert_eq!(Encoding::Gbk.encode("中").unwrap(), [0xD6, 0xD0]);
    assert_eq!(
        Encoding::Windows1251
            .decode(&[0xCF, 0xF0, 0xE8, 0xE2, 0xE5, 0xF2])
            .unwrap(),
        "Привет"
    );
}

#[test]
fn legacy_encodings_round_trip_representable_text() {
    let samples = [
        (Encoding::Utf8, "aé中😀"),
        (Encoding::MacRoman, "é"),
        (Encoding::Windows1252, "é€"),
        (Encoding::Gbk, "中"),
        (Encoding::IsoLatin2, "ř"),
        (Encoding::Windows1250, "ř"),
        (Encoding::Windows1251, "Привет"),
        (Encoding::Windows1253, "Ελλάδα"),
        (Encoding::Windows1254, "İstanbul"),
        (Encoding::EucJp, "日本語"),
        (Encoding::ShiftJis, "日本語"),
        (Encoding::Iso2022Jp, "日本語"),
    ];

    for (encoding, text) in samples {
        assert_eq!(
            encoding.decode(&encoding.encode(text).unwrap()).unwrap(),
            text,
            "{encoding:?}"
        );
    }
}

#[test]
fn explicit_endianness_is_always_used_and_has_no_bom() {
    assert_eq!(Encoding::Utf16Be.decode(&[0x00, 0x61]).unwrap(), "a");
    assert_eq!(Encoding::Utf16Le.decode(&[0x61, 0x00]).unwrap(), "a");
    assert_eq!(Encoding::Utf16Be.encode("a").unwrap(), [0x00, 0x61]);
    assert_eq!(Encoding::Utf16Le.encode("a").unwrap(), [0x61, 0x00]);

    assert_eq!(
        Encoding::Utf32Be.decode(&[0x00, 0x00, 0x00, 0x61]).unwrap(),
        "a"
    );
    assert_eq!(
        Encoding::Utf32Le.decode(&[0x61, 0x00, 0x00, 0x00]).unwrap(),
        "a"
    );
    assert_eq!(
        Encoding::Utf32Be.encode("a").unwrap(),
        [0x00, 0x00, 0x00, 0x61]
    );
    assert_eq!(
        Encoding::Utf32Le.encode("a").unwrap(),
        [0x61, 0x00, 0x00, 0x00]
    );
}

#[test]
fn generic_utf16_detects_the_bom_and_defaults_to_little_endian() {
    assert_eq!(
        Encoding::Utf16.decode(&[0xFE, 0xFF, 0x00, 0x61]).unwrap(),
        "a"
    );
    assert_eq!(
        Encoding::Utf16.decode(&[0xFF, 0xFE, 0x61, 0x00]).unwrap(),
        "a"
    );
    assert_eq!(Encoding::Utf16.decode(&[0x61, 0x00]).unwrap(), "a");
    assert_eq!(
        Encoding::Utf16.encode("a").unwrap(),
        [0xFF, 0xFE, 0x61, 0x00]
    );
}

#[test]
fn generic_utf32_detects_the_bom_and_defaults_to_little_endian() {
    assert_eq!(
        Encoding::Utf32
            .decode(&[0x00, 0x00, 0xFE, 0xFF, 0x00, 0x00, 0x00, 0x61])
            .unwrap(),
        "a"
    );
    assert_eq!(
        Encoding::Utf32
            .decode(&[0xFF, 0xFE, 0x00, 0x00, 0x61, 0x00, 0x00, 0x00])
            .unwrap(),
        "a"
    );
    assert_eq!(
        Encoding::Utf32.decode(&[0x61, 0x00, 0x00, 0x00]).unwrap(),
        "a"
    );
    assert_eq!(
        Encoding::Utf32.encode("a").unwrap(),
        [0xFF, 0xFE, 0x00, 0x00, 0x61, 0x00, 0x00, 0x00]
    );
}

#[test]
fn utf16_rejects_odd_lengths_and_unpaired_surrogates() {
    assert_eq!(
        Encoding::Utf16.decode(&[0x61]),
        Err(CsvError::CannotDecodeData)
    );
    assert_eq!(
        Encoding::Utf16Be.decode(&[0x00, 0x61, 0x00]),
        Err(CsvError::CannotDecodeData)
    );
    // A lone high surrogate, a lone low surrogate, and a high surrogate
    // followed by a non-low-surrogate unit.
    assert_eq!(
        Encoding::Utf16Le.decode(&[0x3D, 0xD8]),
        Err(CsvError::CannotDecodeData)
    );
    assert_eq!(
        Encoding::Utf16Le.decode(&[0x00, 0xDE]),
        Err(CsvError::CannotDecodeData)
    );
    assert_eq!(
        Encoding::Utf16Le.decode(&[0x3D, 0xD8, 0x61, 0x00]),
        Err(CsvError::CannotDecodeData)
    );
}

#[test]
fn utf32_rejects_odd_lengths_invalid_code_points_and_surrogates() {
    assert_eq!(
        Encoding::Utf32.decode(&[0x61, 0x00, 0x00]),
        Err(CsvError::CannotDecodeData)
    );
    assert_eq!(
        Encoding::Utf32Le.decode(&[0x00, 0x00, 0x11, 0x00]),
        Err(CsvError::CannotDecodeData)
    );
    assert_eq!(
        Encoding::Utf32Be.decode(&[0x00, 0x11, 0x00, 0x00]),
        Err(CsvError::CannotDecodeData)
    );
    // U+D800, a surrogate, is not a scalar value.
    assert_eq!(
        Encoding::Utf32Le.decode(&[0x00, 0xD8, 0x00, 0x00]),
        Err(CsvError::CannotDecodeData)
    );
    // U+10FFFF is the highest scalar value and must decode.
    assert_eq!(
        Encoding::Utf32Le.decode(&[0xFF, 0xFF, 0x10, 0x00]).unwrap(),
        "\u{10FFFF}"
    );
}
