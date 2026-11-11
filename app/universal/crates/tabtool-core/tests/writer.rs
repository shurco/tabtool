//! Ports of the Apple `CSVWriterTests` suite and the fixture round-trip test
//! from `RoundTripTests`, plus the Review Focus cases.
//!
//! The three random round-trip tests need the seeded generator and belong to
//! Task 13; the fixture round-trip needs no generator and is ported here.

use tabtool_core::{
    CsvConfiguration, CsvError, CsvParser, CsvTable, CsvWriteIssue, CsvWriter, is_ambiguous, issue,
};
use tabtool_testdata::fixture;

fn rows(values: &[&[&str]]) -> Vec<Vec<String>> {
    values
        .iter()
        .map(|row| row.iter().map(|cell| (*cell).to_owned()).collect())
        .collect()
}

fn headers(values: &[&str]) -> Option<Vec<String>> {
    Some(values.iter().map(|name| (*name).to_owned()).collect())
}

/// Mirrors Swift's `CSVTable(rows:headers:)` default minimum of one column.
fn table(rows: Vec<Vec<String>>, headers: Option<Vec<String>>) -> CsvTable {
    CsvTable::from_rows(rows, headers, 1)
}

fn no_quote() -> CsvConfiguration {
    CsvConfiguration {
        quote_character: String::new(),
        ..CsvConfiguration::default()
    }
}

fn parse_all(data: &[u8], configuration: &CsvConfiguration) -> Vec<Vec<String>> {
    CsvParser::from_bytes(data, configuration)
        .unwrap()
        .read_all()
        .unwrap()
}

fn cells(table: &CsvTable) -> Vec<&Vec<String>> {
    table.columns.iter().map(|column| &column.cells).collect()
}

#[test]
fn quotes_every_field_and_omits_trailing_newline() {
    let t = table(rows(&[&["a", "b"]]), headers(&["h1", "h2"]));
    let header = headers(&["h1", "h2"]);
    let text = CsvWriter::new(&t, header.as_deref(), &CsvConfiguration::default())
        .string()
        .unwrap();
    assert_eq!(text, "\"h1\",\"h2\"\n\"a\",\"b\"");
}

#[test]
fn rows_join_with_line_feed_only() {
    let t = table(rows(&[&["a"], &["b"]]), None);
    let text = CsvWriter::new(&t, None, &CsvConfiguration::default())
        .string()
        .unwrap();
    assert_eq!(text, "\"a\"\n\"b\"");
}

#[test]
fn writer_uses_visual_column_order() {
    let mut t = table(rows(&[&["a", "b"]]), None);
    t.move_column(1, 0);
    let text = CsvWriter::new(&t, None, &CsvConfiguration::default())
        .string()
        .unwrap();
    assert_eq!(text, "\"b\",\"a\"");
}

#[test]
fn doubles_quotes_with_quote_escape() {
    let t = table(rows(&[&["a\"b"]]), None);
    let text = CsvWriter::new(&t, None, &CsvConfiguration::default())
        .string()
        .unwrap();
    assert_eq!(text, r#""a""b""#);
}

#[test]
fn uses_backslash_escape() {
    let t = table(rows(&[&["a\"b"]]), None);
    let configuration = CsvConfiguration {
        escape_character: "\\".to_string(),
        ..CsvConfiguration::default()
    };
    let text = CsvWriter::new(&t, None, &configuration).string().unwrap();
    assert_eq!(text, r#""a\"b""#);
}

#[test]
fn doubles_escape_character() {
    let t = table(rows(&[&["a\\b"]]), None);
    let configuration = CsvConfiguration {
        escape_character: "\\".to_string(),
        ..CsvConfiguration::default()
    };
    let text = CsvWriter::new(&t, None, &configuration).string().unwrap();
    assert_eq!(text, r#""a\\b""#);
    assert_eq!(
        parse_all(text.as_bytes(), &configuration),
        rows(&[&["a\\b"]])
    );
}

#[test]
fn quoted_newline_survives() {
    let t = table(rows(&[&["a\nb"]]), None);
    let text = CsvWriter::new(&t, None, &CsvConfiguration::default())
        .string()
        .unwrap();
    assert_eq!(text, "\"a\nb\"");
}

#[test]
fn no_quote_ambiguity_throws_unless_allowed() {
    let t = table(rows(&[&["a,b"]]), None);
    let configuration = no_quote();

    assert_eq!(
        CsvWriter::new(&t, None, &configuration).string(),
        Err(CsvError::AmbiguousOutput)
    );

    let mut writer = CsvWriter::new(&t, None, &configuration);
    writer.allow_ambiguous_output = true;
    assert_eq!(writer.string().unwrap(), "a,b");
}

#[test]
fn no_quote_line_break_is_ambiguous() {
    let t = table(rows(&[&["a\nb"]]), None);
    let configuration = no_quote();

    assert_eq!(
        issue(&t, None, &configuration),
        Some(CsvWriteIssue::AmbiguousOutput)
    );
    assert_eq!(
        CsvWriter::new(&t, None, &configuration).string(),
        Err(CsvError::AmbiguousOutput)
    );
}

#[test]
fn no_quote_every_row_break_is_ambiguous() {
    for row_break in ["\r", "\r\n", "\u{0085}", "\u{2028}", "\u{2029}"] {
        let t = table(rows(&[&[&format!("a{row_break}b")]]), None);
        let configuration = no_quote();
        assert_eq!(
            issue(&t, None, &configuration),
            Some(CsvWriteIssue::AmbiguousOutput),
            "{row_break:?}"
        );
        assert_eq!(
            CsvWriter::new(&t, None, &configuration).string(),
            Err(CsvError::AmbiguousOutput),
            "{row_break:?}"
        );
    }
}

#[test]
fn no_quote_trailing_line_feed_throws() {
    let t = table(rows(&[&["a"], &[""]]), None);
    let configuration = no_quote();
    let writer = CsvWriter::new(&t, None, &configuration);

    assert_eq!(writer.string(), Err(CsvError::AmbiguousOutput));
    assert_eq!(writer.data(), Err(CsvError::AmbiguousOutput));
}

#[test]
fn no_quote_trailing_line_feed_allowed() {
    let t = table(rows(&[&["a"], &[""]]), None);
    let configuration = no_quote();
    let mut writer = CsvWriter::new(&t, None, &configuration);
    writer.allow_ambiguous_output = true;
    assert_eq!(writer.string().unwrap(), "a\n");
}

#[test]
fn validation_detects_trailing_line_feed() {
    let t = table(rows(&[&["a"], &[""]]), None);
    assert_eq!(
        issue(&t, None, &no_quote()),
        Some(CsvWriteIssue::AmbiguousOutput)
    );
}

#[test]
fn no_quote_last_empty_row_with_header_is_ambiguous() {
    let t = table(rows(&[&["x"], &[""]]), headers(&["h"]));
    let header = headers(&["h"]);
    let configuration = no_quote();

    assert_eq!(
        issue(&t, header.as_deref(), &configuration),
        Some(CsvWriteIssue::AmbiguousOutput)
    );
    assert_eq!(
        CsvWriter::new(&t, header.as_deref(), &configuration).string(),
        Err(CsvError::AmbiguousOutput)
    );
}

#[test]
fn single_empty_cell_is_not_ambiguous() {
    let t = table(rows(&[&[""]]), None);
    let configuration = no_quote();
    let writer = CsvWriter::new(&t, None, &configuration);

    assert_eq!(writer.string().unwrap(), "");
    assert_eq!(issue(&t, None, &configuration), None);
    assert!(!is_ambiguous(&t, None, &configuration));

    // The empty output parses back to a single empty cell.
    let reparsed = table(parse_all(b"", &configuration), None);
    assert_eq!(reparsed.row_count(), 1);
    assert_eq!(reparsed.columns.len(), 1);
    assert_eq!(reparsed.cell(0, 0), "");
}

#[test]
fn unencodable_text_throws() {
    let t = table(rows(&[&["Привет"]]), None);
    let configuration = CsvConfiguration {
        encoding: tabtool_core::Encoding::Windows1252,
        ..CsvConfiguration::default()
    };
    let writer = CsvWriter::new(&t, None, &configuration);

    assert!(writer.string().is_ok());
    assert_eq!(writer.data(), Err(CsvError::UnencodableText));
}

#[test]
fn utf8_bom_is_prepended() {
    let t = table(rows(&[&["a"]]), None);
    let configuration = CsvConfiguration::default();
    let mut writer = CsvWriter::new(&t, None, &configuration);
    writer.include_utf8_bom = true;

    let mut expected = vec![0xEF, 0xBB, 0xBF];
    expected.extend_from_slice(b"\"a\"");
    assert_eq!(writer.data().unwrap(), expected);
}

#[test]
fn utf8_bom_is_only_prepended_for_utf8() {
    let t = table(rows(&[&["a"]]), None);
    let configuration = CsvConfiguration {
        encoding: tabtool_core::Encoding::Windows1252,
        ..no_quote()
    };
    let mut writer = CsvWriter::new(&t, None, &configuration);
    writer.include_utf8_bom = true;

    assert_eq!(writer.data().unwrap(), b"a");
}

#[test]
fn encodes_with_configured_encoding() {
    let t = table(rows(&[&["café"]]), None);
    let configuration = CsvConfiguration {
        encoding: tabtool_core::Encoding::Windows1252,
        ..no_quote()
    };
    assert_eq!(
        CsvWriter::new(&t, None, &configuration).data().unwrap(),
        [0x63, 0x61, 0x66, 0xE9]
    );
}

#[test]
fn writer_does_not_convert_decimal_mark() {
    let t = table(rows(&[&["1,5"]]), None);
    let configuration = CsvConfiguration {
        decimal_mark: ".".to_string(),
        ..CsvConfiguration::default()
    };
    let text = CsvWriter::new(&t, None, &configuration).string().unwrap();
    assert_eq!(text, "\"1,5\"");
}

#[test]
fn writes_header_when_there_are_no_rows() {
    let t = table(Vec::new(), headers(&["h1", "h2"]));
    let header = t.headers();
    let text = CsvWriter::new(&t, header.as_deref(), &CsvConfiguration::default())
        .string()
        .unwrap();
    assert_eq!(text, "\"h1\",\"h2\"");
}

#[test]
fn empty_headerless_table_writes_empty_output() {
    let t = CsvTable::new(Vec::new());
    let configuration = CsvConfiguration::default();
    let writer = CsvWriter::new(&t, None, &configuration);

    assert_eq!(writer.string().unwrap(), "");
    assert!(writer.data().unwrap().is_empty());
}

#[test]
fn validation_detects_ambiguity_and_encoding() {
    let ambiguous = table(rows(&[&["a,b"]]), None);
    let cyrillic = table(rows(&[&["Привет"]]), None);

    assert_eq!(
        issue(&ambiguous, None, &no_quote()),
        Some(CsvWriteIssue::AmbiguousOutput)
    );
    assert_eq!(
        issue(
            &cyrillic,
            None,
            &CsvConfiguration {
                encoding: tabtool_core::Encoding::Windows1252,
                ..CsvConfiguration::default()
            }
        ),
        Some(CsvWriteIssue::UnencodableText)
    );
    assert_eq!(issue(&ambiguous, None, &CsvConfiguration::default()), None);
}

#[test]
fn validation_checks_header_ambiguity() {
    let t = table(rows(&[&["x"]]), None);
    let header = ["a,b".to_string()];

    assert_eq!(
        issue(&t, Some(&header), &no_quote()),
        Some(CsvWriteIssue::AmbiguousOutput)
    );
}

#[test]
fn validation_checks_header_encoding() {
    let t = table(rows(&[&["x"]]), None);
    let header = ["Привет".to_string()];
    let configuration = CsvConfiguration {
        encoding: tabtool_core::Encoding::Windows1252,
        ..CsvConfiguration::default()
    };

    assert_eq!(
        issue(&t, Some(&header), &configuration),
        Some(CsvWriteIssue::UnencodableText)
    );
}

#[test]
fn validation_matches_writer_issue_order() {
    let t = table(rows(&[&["Привет, x"]]), None);
    let configuration = CsvConfiguration {
        encoding: tabtool_core::Encoding::Windows1252,
        ..no_quote()
    };

    assert_eq!(
        issue(&t, None, &configuration),
        Some(CsvWriteIssue::AmbiguousOutput)
    );
    assert_eq!(
        CsvWriter::new(&t, None, &configuration).data(),
        Err(CsvError::AmbiguousOutput)
    );
}

#[test]
fn renamed_header_with_separator_and_quotes_round_trips() {
    let mut t = table(rows(&[&["1", "2"]]), headers(&["a", "b"]));
    let name = "x,\"y\"\nz";
    assert!(t.rename_header(name, 0));

    let header = t.headers();
    let configuration = CsvConfiguration::default();
    let data = CsvWriter::new(&t, header.as_deref(), &configuration)
        .data()
        .unwrap();
    let parsed = parse_all(&data, &configuration);

    assert_eq!(parsed.first(), Some(&rows(&[&[name, "b"]])[0]));
}

#[test]
fn fixture_round_trip_preserves_logical_table() {
    let cases = [
        ("comma-separated", no_quote()),
        (
            "semicolon-separated",
            CsvConfiguration {
                column_separator: ";".to_string(),
                quote_character: String::new(),
                decimal_mark: ",".to_string(),
                ..CsvConfiguration::default()
            },
        ),
        (
            "comma-separated-quote",
            CsvConfiguration {
                decimal_mark: ",".to_string(),
                ..CsvConfiguration::default()
            },
        ),
        ("quote-quote-escape", CsvConfiguration::default()),
        (
            "quote-backslash-escape",
            CsvConfiguration {
                escape_character: "\\".to_string(),
                ..CsvConfiguration::default()
            },
        ),
        ("blank-lines", no_quote()),
    ];

    for (name, configuration) in cases {
        let parsed = parse_all(&fixture(&format!("Reading/{name}.csv")), &configuration);
        let header = if configuration.first_row_as_header {
            parsed.first().cloned()
        } else {
            None
        };
        let body = if configuration.first_row_as_header {
            parsed[1..].to_vec()
        } else {
            parsed
        };
        let table = CsvTable::from_rows(body, header.clone(), 1);

        let data = CsvWriter::new(&table, header.as_deref(), &configuration)
            .data()
            .unwrap();
        let reparsed = parse_all(&data, &configuration);
        let new_header = if configuration.first_row_as_header {
            reparsed.first().cloned()
        } else {
            None
        };
        let new_body = if configuration.first_row_as_header {
            reparsed[1..].to_vec()
        } else {
            reparsed
        };
        let new_table = CsvTable::from_rows(new_body, new_header, 1);

        assert_eq!(new_table.headers(), table.headers(), "{name}");
        assert_eq!(cells(&new_table), cells(&table), "{name}");
    }
}
