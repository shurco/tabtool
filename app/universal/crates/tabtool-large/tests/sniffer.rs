//! Ported from `LargeFileSnifferTests.swift`, plus the sample-cut boundaries:
//! partial multibyte text, a cut inside a quoted field and one row longer than
//! the sample.

use std::fs;

use tabtool_core::{CsvConfiguration, CsvError, Encoding};
use tabtool_large::{LargeFile, MappedFile, check, is_byte_scannable, sniff, validate};

/// Maps `bytes` as a temporary CSV file and calls `body` with it.
fn with_mapped<R>(bytes: &[u8], body: impl FnOnce(&MappedFile) -> R) -> R {
    let directory = tempfile::tempdir().expect("temp directory is created");
    let path = directory.path().join("s.csv");
    fs::write(&path, bytes).expect("sample is written");
    let file = MappedFile::open(&path).expect("sample opens");
    body(&file)
}

fn sniff_bytes(bytes: &[u8]) -> Result<CsvConfiguration, CsvError> {
    sniff_bytes_with(bytes, false)
}

fn sniff_bytes_with(
    bytes: &[u8],
    prefers_chinese_encoding: bool,
) -> Result<CsvConfiguration, CsvError> {
    with_mapped(bytes, |file| sniff(file, prefers_chinese_encoding))
}

#[test]
fn detects_utf8_comma() {
    let configuration = sniff_bytes(b"name,qty\nwidget,3\ngadget,4\n").expect("sniffs");
    assert_eq!(configuration.encoding, Encoding::Utf8);
    assert_eq!(configuration.column_separator, ",");
    assert!(configuration.first_row_as_header);
}

#[test]
fn detects_semicolon_with_decimal_comma() {
    let configuration = sniff_bytes(b"a;b\n1,5;2,5\n3,5;4,5\n").expect("sniffs");
    assert_eq!(configuration.column_separator, ";");
    assert_eq!(configuration.decimal_mark, ",");
}

#[test]
fn detects_tabs() {
    let configuration = sniff_bytes(b"a\tb\n1\t2\n3\t4\n").expect("sniffs");
    assert_eq!(configuration.column_separator, "\t");
}

#[test]
fn accepts_cp1252() {
    let bytes = [
        0x63, 0x61, 0x66, 0xE9, 0x2C, 0x31, 0x0A, 0x74, 0x65, 0xE9, 0x2C, 0x32, 0x0A,
    ];
    let configuration = sniff_bytes(&bytes).expect("sniffs");
    assert_eq!(configuration.encoding, Encoding::Windows1252);
}

#[test]
fn empty_file_sniffs_to_defaults() {
    assert_eq!(sniff_bytes(b"").expect("sniffs").encoding, Encoding::Utf8);
}

#[test]
fn refuses_non_byte_scannable_files() {
    let cases: [&[u8]; 5] = [
        &[0xFF, 0xFE, 0x61, 0x00, 0x2C, 0x00, 0x62, 0x00],
        &[0xFE, 0xFF, 0x00, 0x61, 0x00, 0x2C, 0x00, 0x62],
        &[0x00, 0x00, 0xFE, 0xFF, 0x00, 0x00, 0x00, 0x61],
        &[0x61, 0x00, 0x2C, 0x00, 0x62, 0x00, 0x0A, 0x00],
        &[
            0x61, 0x2C, 0x1B, 0x24, 0x42, 0x30, 0x6C, 0x1B, 0x28, 0x42, 0x0A,
        ],
    ];
    for bytes in cases {
        assert_eq!(
            sniff_bytes(bytes),
            Err(CsvError::UnsupportedLargeFileEncoding),
            "{bytes:02X?}"
        );
    }
}

#[test]
fn partial_multibyte_at_sample_cut() {
    let row = "é,é\n".as_bytes();
    assert!(!LargeFile::SAMPLE_BYTES.is_multiple_of(row.len()));
    let mut bytes = Vec::new();
    while bytes.len() < LargeFile::SAMPLE_BYTES + 100 {
        bytes.extend_from_slice(row);
    }
    let configuration = sniff_bytes(&bytes).expect("sniffs");
    assert_eq!(configuration.encoding, Encoding::Utf8);
}

/// The 1 MiB cut can land inside a multibyte scalar. The partial scalar must
/// be trimmed so strict UTF-8 decoding still succeeds.
#[test]
fn incomplete_multibyte_at_the_sample_cut_is_trimmed() {
    let mut bytes = vec![b'a'; LargeFile::SAMPLE_BYTES - 1];
    bytes.extend_from_slice("é".as_bytes());
    assert!(bytes.len() > LargeFile::SAMPLE_BYTES);

    let configuration = sniff_bytes(&bytes).expect("sniffs");
    assert_eq!(configuration.encoding, Encoding::Utf8);
    assert_eq!(configuration.column_separator, ",");

    with_mapped(&bytes, |file| {
        assert_eq!(check(file, &CsvConfiguration::default()), Ok(()));
    });
}

#[test]
fn sample_cut_inside_quoted_multiline_field() {
    let mut bytes = "x,y\n".repeat(262_143).into_bytes();
    bytes.extend_from_slice(b"\"ab\ncd\",z\n");
    bytes.extend_from_slice(&"x,y\n".repeat(100).into_bytes());
    let configuration = sniff_bytes(&bytes).expect("sniffs");
    assert_eq!(configuration.column_separator, ",");
}

#[test]
fn strict_parse_failure_beyond_the_detector_rows_surfaces() {
    let result = sniff_bytes(b"a,b\n1,2\n3,4\n5,6\n7,8\n9,10\nx\"y\"z,1\n");
    assert_eq!(result, Err(CsvError::QuoteInUnquotedField));
}

#[test]
fn unbalanced_quote_falls_back_to_quoteless() {
    let configuration = sniff_bytes(b"a,b\n\"x,y\n1,2\n").expect("sniffs");
    assert_eq!(configuration.quote_character, "");
}

#[test]
fn rejects_multibyte_delimiters() {
    let configurations = [
        CsvConfiguration {
            column_separator: "§".to_owned(),
            ..CsvConfiguration::default()
        },
        CsvConfiguration {
            column_separator: "||".to_owned(),
            ..CsvConfiguration::default()
        },
        CsvConfiguration {
            quote_character: "\u{201C}".to_owned(),
            escape_character: "\u{201C}".to_owned(),
            ..CsvConfiguration::default()
        },
        CsvConfiguration {
            escape_character: "¦".to_owned(),
            ..CsvConfiguration::default()
        },
    ];
    for configuration in configurations {
        assert_eq!(
            validate(&configuration),
            Err(CsvError::UnsupportedLargeFileDelimiter),
            "{configuration:?}"
        );
    }
}

#[test]
fn accepts_ascii_configurations() {
    assert!(validate(&CsvConfiguration::default()).is_ok());
    let configuration = CsvConfiguration {
        column_separator: "|".to_owned(),
        quote_character: String::new(),
        escape_character: "\\".to_owned(),
        ..CsvConfiguration::default()
    };
    assert!(validate(&configuration).is_ok());
}

#[test]
fn validate_rejects_refused_encodings() {
    let configuration = CsvConfiguration {
        encoding: Encoding::Utf16,
        ..CsvConfiguration::default()
    };
    assert_eq!(
        validate(&configuration),
        Err(CsvError::UnsupportedLargeFileEncoding)
    );
}

#[test]
fn byte_scannable_set() {
    let accepted = [
        Encoding::Utf8,
        Encoding::MacRoman,
        Encoding::Windows1252,
        Encoding::IsoLatin2,
        Encoding::Windows1250,
        Encoding::Windows1251,
        Encoding::Windows1253,
        Encoding::Windows1254,
        Encoding::Gbk,
    ];
    for encoding in accepted {
        assert!(is_byte_scannable(encoding), "{encoding:?}");
    }
    let refused = [
        Encoding::Utf16,
        Encoding::Utf16Be,
        Encoding::Utf16Le,
        Encoding::Utf32,
        Encoding::Utf32Be,
        Encoding::Utf32Le,
        Encoding::ShiftJis,
        Encoding::EucJp,
        Encoding::Iso2022Jp,
    ];
    for encoding in refused {
        assert!(!is_byte_scannable(encoding), "{encoding:?}");
    }
}

#[test]
fn check_accepts_an_explicit_configuration() {
    with_mapped(b"a;b\n1;2\n", |file| {
        let configuration = CsvConfiguration {
            column_separator: ";".to_owned(),
            ..CsvConfiguration::default()
        };
        assert_eq!(check(file, &configuration), Ok(()));
    });
}

#[test]
fn check_refuses_unsupported_encoding_and_malformed_sample() {
    with_mapped(b"a,\"b\nc", |file| {
        let configuration = CsvConfiguration {
            encoding: Encoding::Utf16,
            ..CsvConfiguration::default()
        };
        assert_eq!(
            check(file, &configuration),
            Err(CsvError::UnsupportedLargeFileEncoding)
        );
        assert_eq!(
            check(file, &CsvConfiguration::default()),
            Err(CsvError::MalformedQuotedField)
        );
    });
}

#[test]
fn check_refuses_a_file_that_looks_like_utf16() {
    with_mapped(&[0xFF, 0xFE, 0x61, 0x00, 0x2C, 0x00], |file| {
        assert_eq!(
            check(file, &CsvConfiguration::default()),
            Err(CsvError::UnsupportedLargeFileEncoding)
        );
    });
}

#[test]
fn chinese_preference_selects_gbk() {
    let bytes = Encoding::Gbk
        .encode("姓名,数量\n张三,3\n李四,4\n")
        .expect("sample encodes");
    assert_eq!(
        sniff_bytes_with(&bytes, false).expect("sniffs").encoding,
        Encoding::Windows1252
    );
    assert_eq!(
        sniff_bytes_with(&bytes, true).expect("sniffs").encoding,
        Encoding::Gbk
    );
}

#[test]
fn single_row_longer_than_the_sample_is_parsed_as_is() {
    let bytes = vec![b'a'; LargeFile::SAMPLE_BYTES + 100];
    let configuration = sniff_bytes(&bytes).expect("one long row parses as is");
    assert_eq!(configuration.encoding, Encoding::Utf8);
    assert_eq!(configuration.column_separator, ",");
}

#[test]
fn long_quoted_row_is_malformed_for_an_explicit_configuration() {
    let mut bytes = vec![b'"'];
    bytes.resize(LargeFile::SAMPLE_BYTES + 100, b'a');
    with_mapped(&bytes, |file| {
        assert_eq!(
            check(file, &CsvConfiguration::default()),
            Err(CsvError::MalformedQuotedField)
        );
    });
}
