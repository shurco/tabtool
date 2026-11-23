//! Ports of the Apple `CSVParserTests` suite, plus the Review Focus cases.

use tabtool_core::{CsvConfiguration, CsvError, CsvParser, LineEnding, Terminator, is_numeric};
use tabtool_testdata::fixture;

/// Parses a fixture from the `Reading` folder, like Apple's `fixture(_:)`,
/// which searches `Reading` before the other fixture folders.
fn parse_fixture(
    name: &str,
    configuration: &CsvConfiguration,
) -> Result<Vec<Vec<String>>, CsvError> {
    parse_all(&fixture(&format!("Reading/{name}.csv")), configuration)
}

fn parse_all(data: &[u8], configuration: &CsvConfiguration) -> Result<Vec<Vec<String>>, CsvError> {
    CsvParser::from_bytes(data, configuration)?.read_all()
}

fn parse_text(text: &str, configuration: &CsvConfiguration) -> Result<Vec<Vec<String>>, CsvError> {
    CsvParser::from_string(text, configuration).read_all()
}

fn rows(values: &[&[&str]]) -> Vec<Vec<String>> {
    values
        .iter()
        .map(|row| row.iter().map(|cell| cell.to_string()).collect())
        .collect()
}

#[test]
fn reads_unquoted_rows() {
    let cases = [
        (
            "comma-separated",
            CsvConfiguration {
                quote_character: String::new(),
                ..CsvConfiguration::default()
            },
            3,
            3,
        ),
        (
            "semicolon-separated",
            CsvConfiguration {
                column_separator: ";".to_string(),
                quote_character: String::new(),
                decimal_mark: ",".to_string(),
                ..CsvConfiguration::default()
            },
            3,
            3,
        ),
    ];

    for (name, configuration, row_count, field_count) in cases {
        let parsed = parse_fixture(name, &configuration).unwrap();
        assert_eq!(parsed.len(), row_count, "{name}");
        assert!(
            parsed.iter().all(|row| row.len() == field_count),
            "{name}: {parsed:?}"
        );
        for cell in parsed.last().unwrap() {
            assert!(
                is_numeric(cell, &configuration.decimal_mark),
                "{name}: {cell:?}"
            );
        }
    }
}

#[test]
fn quoted_rows_use_decimal_comma() {
    let configuration = CsvConfiguration {
        decimal_mark: ",".to_string(),
        ..CsvConfiguration::default()
    };
    let parsed = parse_fixture("comma-separated-quote", &configuration).unwrap();

    assert_eq!(parsed.len(), 3);
    assert!(parsed.iter().all(|row| row.len() == 3));
    for row in parsed.iter().skip(1) {
        for cell in row {
            assert!(is_numeric(cell, ","), "{cell:?}");
        }
    }
}

#[test]
fn quoted_field_preserves_separator_and_newline() {
    let parsed = parse_text("a,\"b,c\nd\",e", &CsvConfiguration::default()).unwrap();
    assert_eq!(parsed, rows(&[&["a", "b,c\nd", "e"]]));
}

#[test]
fn blank_lines_become_empty_rows() {
    let parsed = parse_fixture("blank-lines", &CsvConfiguration::default()).unwrap();

    assert_eq!(parsed.len(), 6);
    assert_eq!(parsed[2], rows(&[&[""]])[0]);
    assert_eq!(parsed[3], rows(&[&[""]])[0]);
    assert_eq!(parsed.first().unwrap(), &rows(&[&["10"]])[0]);
    assert_eq!(parsed.last().unwrap(), &rows(&[&["60"]])[0]);
}

#[test]
fn empty_input_returns_one_empty_row() {
    assert_eq!(
        parse_all(b"", &CsvConfiguration::default()).unwrap(),
        rows(&[&[""]])
    );
}

#[test]
fn trailing_empty_fields_are_preserved() {
    let configuration = CsvConfiguration::default();
    assert_eq!(
        parse_all(b"a,b,\n", &configuration).unwrap(),
        rows(&[&["a", "b", ""]])
    );
    assert_eq!(
        parse_all(b"a,b,,\n", &configuration).unwrap(),
        rows(&[&["a", "b", "", ""]])
    );
}

#[test]
fn crlf_and_cr_are_single_row_breaks() {
    let parsed = parse_all(b"a,b\r\nc,d\re,f\n", &CsvConfiguration::default()).unwrap();
    assert_eq!(parsed, rows(&[&["a", "b"], &["c", "d"], &["e", "f"]]));
}

#[test]
fn unicode_line_separators_break_rows() {
    let parsed = parse_all(
        "a,b\u{0085}c,d\u{2028}e,f\u{2029}g,h".as_bytes(),
        &CsvConfiguration::default(),
    )
    .unwrap();
    assert_eq!(
        parsed,
        rows(&[&["a", "b"], &["c", "d"], &["e", "f"], &["g", "h"]])
    );
}

#[test]
fn string_initializer_and_reset() {
    let configuration = CsvConfiguration {
        column_separator: ";".to_string(),
        ..CsvConfiguration::default()
    };
    let mut parser = CsvParser::from_string("a;b\nc;d", &configuration);

    assert!(!parser.had_utf8_bom());
    assert_eq!(parser.next_row().unwrap().unwrap(), rows(&[&["a", "b"]])[0]);
    assert!(!parser.is_at_end());
    assert_eq!(parser.next_row().unwrap().unwrap(), rows(&[&["c", "d"]])[0]);
    assert!(parser.is_at_end());
    assert_eq!(parser.next_row().unwrap(), None);

    parser.reset();
    assert!(!parser.is_at_end());
    assert_eq!(
        parser.read_all().unwrap(),
        rows(&[&["a", "b"], &["c", "d"]])
    );
}

#[test]
fn malformed_quoted_fixtures_throw() {
    let escaped = CsvConfiguration {
        escape_character: "\\".to_string(),
        ..CsvConfiguration::default()
    };
    let cases = [
        ("missing-quote-atEnd", CsvConfiguration::default()),
        ("tooMany-quotes-atEnd", CsvConfiguration::default()),
        ("missing-backslash-beforeValue", escaped.clone()),
        ("missing-backslash-afterValue", escaped.clone()),
        ("missing-value-for-backslash-inquote", escaped.clone()),
        ("missing-value-for-backslash", escaped.clone()),
    ];

    for (name, configuration) in cases {
        assert_eq!(
            parse_fixture(name, &configuration),
            Err(CsvError::MalformedQuotedField),
            "{name}"
        );
    }
}

#[test]
fn quote_in_unquoted_value_throws() {
    assert_eq!(
        parse_fixture("quote-in-unquoted-value", &CsvConfiguration::default()),
        Err(CsvError::QuoteInUnquotedField)
    );
}

#[test]
fn content_after_closing_quote_throws() {
    assert_eq!(
        parse_all(b"\"a\"b,c", &CsvConfiguration::default()),
        Err(CsvError::MalformedQuotedField)
    );
}

#[test]
fn invalid_utf8_throws_cannot_decode() {
    assert!(matches!(
        CsvParser::from_bytes(&[0xFF, 0xFE, 0xFD], &CsvConfiguration::default()),
        Err(CsvError::CannotDecodeData)
    ));
}

#[test]
fn doubled_quote_escape_decodes_exact_cells() {
    let parsed = parse_fixture("quote-quote-escape", &CsvConfiguration::default()).unwrap();

    assert_eq!(
        parsed,
        rows(&[&["sample\" value", "sample"], &["2.4", "2,5"]])
    );
    assert!(is_numeric(&parsed[1][0], "."));
    assert!(!is_numeric(&parsed[1][1], "."));
}

#[test]
fn backslash_escape_decodes_exact_cells() {
    let configuration = CsvConfiguration {
        escape_character: "\\".to_string(),
        ..CsvConfiguration::default()
    };
    let parsed = parse_fixture("quote-backslash-escape", &configuration).unwrap();

    assert_eq!(
        parsed,
        rows(&[&["sample\" value", "sample"], &["2.4", "2,5"]])
    );
}

#[test]
fn no_quote_mode_treats_quotes_as_text() {
    let configuration = CsvConfiguration {
        quote_character: String::new(),
        ..CsvConfiguration::default()
    };
    assert_eq!(
        parse_all(b"a\"b,c", &configuration).unwrap(),
        rows(&[&["a\"b", "c"]])
    );
}

#[test]
fn utf8_bom_is_stripped_and_flagged() {
    let mut data = vec![0xEF, 0xBB, 0xBF];
    data.extend_from_slice(b"a,b");

    let mut parser = CsvParser::from_bytes(&data, &CsvConfiguration::default()).unwrap();
    assert!(parser.had_utf8_bom());
    assert_eq!(parser.read_all().unwrap(), rows(&[&["a", "b"]]));

    // A BOM-only input decodes to empty text and keeps the single empty row.
    let mut bom_only = CsvParser::from_bytes(&data[..3], &CsvConfiguration::default()).unwrap();
    assert!(bom_only.had_utf8_bom());
    assert_eq!(bom_only.read_all().unwrap(), rows(&[&[""]]));

    assert!(!CsvParser::from_string("a,b", &CsvConfiguration::default()).had_utf8_bom());
}

#[test]
fn quoted_fields_keep_each_row_break() {
    let row_breaks = ["\r\n", "\r", "\n", "\u{0085}", "\u{2028}", "\u{2029}"];

    for row_break in row_breaks {
        let text = format!("a,\"b{row_break}c\",d");
        let parsed = parse_text(&text, &CsvConfiguration::default()).unwrap();
        assert_eq!(
            parsed,
            rows(&[&["a", &format!("b{row_break}c"), "d"]]),
            "{row_break:?}"
        );
    }
}

#[test]
fn final_quoted_field_without_closing_quote_is_malformed() {
    for text in ["a,\"b", "a,\"b\n", "a,\"b\r\n", "\"a"] {
        assert_eq!(
            parse_text(text, &CsvConfiguration::default()),
            Err(CsvError::MalformedQuotedField),
            "{text:?}"
        );
    }
}

#[test]
fn quote_in_unquoted_field_is_reported() {
    for text in ["a\"b,c", "\"a\",b\"c"] {
        assert_eq!(
            parse_text(text, &CsvConfiguration::default()),
            Err(CsvError::QuoteInUnquotedField),
            "{text:?}"
        );
    }
}

#[test]
fn trailing_empty_fields_without_row_break_are_preserved() {
    let configuration = CsvConfiguration::default();
    assert_eq!(
        parse_all(b"a,b,", &configuration).unwrap(),
        rows(&[&["a", "b", ""]])
    );
    assert_eq!(
        parse_all(b"a,b,,", &configuration).unwrap(),
        rows(&[&["a", "b", "", ""]])
    );
}

#[test]
fn configuration_uses_the_first_scalar_or_none() {
    let multi_scalar = CsvConfiguration {
        column_separator: ";;".to_string(),
        quote_character: "\"\"".to_string(),
        escape_character: "\"\"".to_string(),
        ..CsvConfiguration::default()
    };
    assert_eq!(
        parse_all(b"\"a\"\"b\";c", &multi_scalar).unwrap(),
        rows(&[&["a\"b", "c"]])
    );

    let absent = CsvConfiguration {
        column_separator: String::new(),
        quote_character: String::new(),
        escape_character: String::new(),
        ..CsvConfiguration::default()
    };
    assert_eq!(
        parse_all(b"a,b\nc", &absent).unwrap(),
        rows(&[&["a,b"], &["c"]])
    );
}

#[test]
fn trailing_row_breaks_do_not_add_rows() {
    let configuration = CsvConfiguration::default();

    // One trailing row break ends the only row.
    assert_eq!(parse_all(b"a\n", &configuration).unwrap(), rows(&[&["a"]]));
    // The second row break is a blank line, which is one empty cell.
    assert_eq!(
        parse_all(b"a\n\n", &configuration).unwrap(),
        rows(&[&["a"], &[""]])
    );
    // Empty input still yields one empty row.
    assert_eq!(parse_all(b"", &configuration).unwrap(), rows(&[&[""]]));
    assert_eq!(
        parse_all(b"a,b\n\n", &configuration).unwrap(),
        rows(&[&["a", "b"], &[""]])
    );
}

fn detect_line_ending(text: &str, quote: &str) -> LineEnding {
    let configuration = CsvConfiguration {
        quote_character: quote.to_owned(),
        ..CsvConfiguration::default()
    };
    let mut parser = CsvParser::from_string(text, &configuration);
    parser.read_all().unwrap();
    parser.line_ending()
}

#[test]
fn line_ending_detects_crlf_and_trailing_break() {
    assert_eq!(
        detect_line_ending("a,b\r\nc,d\r\n", "\""),
        LineEnding {
            terminator: Terminator::Crlf,
            ends_with_terminator: true
        }
    );
}

#[test]
fn line_ending_detects_cr_without_trailing_break() {
    assert_eq!(
        detect_line_ending("a\rb", "\""),
        LineEnding {
            terminator: Terminator::Cr,
            ends_with_terminator: false
        }
    );
}

#[test]
fn line_ending_ignores_breaks_inside_quotes() {
    assert_eq!(
        detect_line_ending("\"x\r\ny\",z\nw", "\""),
        LineEnding {
            terminator: Terminator::Lf,
            ends_with_terminator: false
        }
    );
}

#[test]
fn line_ending_defaults_without_a_row_break() {
    assert_eq!(detect_line_ending("a,b", "\""), LineEnding::default());
}

#[test]
fn line_ending_uses_the_first_terminator() {
    assert_eq!(
        detect_line_ending("a\r\nb\nc", "\""),
        LineEnding {
            terminator: Terminator::Crlf,
            ends_with_terminator: false
        }
    );
}

#[test]
fn line_ending_ignores_nel() {
    assert_eq!(detect_line_ending("a\u{85}b", "\""), LineEnding::default());
}
