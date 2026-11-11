//! Randomized proof of the writer/parser round trip.
//!
//! The generator covers the structures the core must preserve: 0–50 rows,
//! 1–8 columns, headers on or off, and cells drawn from an alphabet that
//! includes separators, quotes, escape characters, every row break the parser
//! recognizes and non-ASCII text.
//!
//! One shape is deliberately excluded from the strict equality: a headerless
//! table with no rows has no distinct serialized form, because empty input is
//! the documented 1×1 table. `empty_headerless_tables_normalize_to_one_cell`
//! pins that normalization.

use proptest::prelude::*;
use tabtool_core::{CsvConfiguration, CsvError, CsvParser, CsvTable, CsvWriter, is_ambiguous};

/// Tokens that stress quoting, escaping, row breaks and encodings.
const TOKENS: &[&str] = &[
    "a",
    "Z",
    "0",
    "1",
    "-1",
    "1.5",
    " ",
    "\t",
    ",",
    ";",
    "\"",
    "'",
    "\\",
    "\r",
    "\n",
    "\r\n",
    "\u{0085}",
    "\u{2028}",
    "\u{2029}",
    "é",
    "e\u{0301}",
    "Привет",
    "你好",
    "日本語",
    "🙂",
    "\u{0000}",
    "\u{FEFF}",
];

/// A quoted configuration exercised by the round-trip property.
struct QuotedCase {
    name: &'static str,
    separator: &'static str,
    quote: &'static str,
    escape: &'static str,
}

/// Quoting every field must survive any cell text.
const QUOTED_CASES: [QuotedCase; 6] = [
    QuotedCase {
        name: "comma, doubled quote",
        separator: ",",
        quote: "\"",
        escape: "\"",
    },
    QuotedCase {
        name: "semicolon, doubled quote",
        separator: ";",
        quote: "\"",
        escape: "\"",
    },
    QuotedCase {
        name: "tab, doubled quote",
        separator: "\t",
        quote: "\"",
        escape: "\"",
    },
    QuotedCase {
        name: "comma, single quote",
        separator: ",",
        quote: "'",
        escape: "'",
    },
    QuotedCase {
        name: "comma, backslash escape",
        separator: ",",
        quote: "\"",
        escape: "\\",
    },
    QuotedCase {
        name: "semicolon, backslash escape",
        separator: ";",
        quote: "\"",
        escape: "\\",
    },
];

/// The unquoted configurations exercised when the writer accepts them.
const UNQUOTED_CASES: [QuotedCase; 2] = [
    QuotedCase {
        name: "unquoted comma",
        separator: ",",
        quote: "",
        escape: "",
    },
    QuotedCase {
        name: "unquoted semicolon",
        separator: ";",
        quote: "",
        escape: "",
    },
];

/// One cell built from zero to four stress tokens.
fn cell_strategy() -> impl Strategy<Value = String> {
    prop::collection::vec(prop::sample::select(TOKENS), 0..=4).prop_map(|tokens| tokens.concat())
}

/// One header name per column.
fn headers_strategy(columns: usize) -> impl Strategy<Value = Vec<String>> {
    prop::collection::vec(cell_strategy(), columns)
}

/// A random table in the shape the property expects.
fn table_strategy() -> impl Strategy<Value = CsvTable> {
    (0usize..=50, 1usize..=8, any::<bool>()).prop_flat_map(
        |(row_count, column_count, headers_present)| {
            let rows = prop::collection::vec(
                prop::collection::vec(cell_strategy(), column_count),
                row_count,
            );
            let headers = headers_strategy(column_count);
            (rows, headers).prop_map(move |(rows, headers)| {
                // A headerless table with no rows writes empty bytes, which the
                // parser turns back into the documented 1×1 table. A header row
                // gives the writer something to emit, so 0-row tables in the
                // strict round trip always carry headers; the headerless case
                // has its own normalization test below.
                let headers = if headers_present || row_count == 0 {
                    Some(headers)
                } else {
                    None
                };
                CsvTable::from_rows(rows, headers, column_count)
            })
        },
    )
}

/// A configuration from a quoted or unquoted case and the table's header state.
fn configuration(case: &QuotedCase, first_row_as_header: bool) -> CsvConfiguration {
    CsvConfiguration {
        column_separator: case.separator.to_owned(),
        quote_character: case.quote.to_owned(),
        escape_character: case.escape.to_owned(),
        first_row_as_header,
        ..CsvConfiguration::default()
    }
}

/// The header row the document would write for this table and configuration.
fn header(table: &CsvTable, configuration: &CsvConfiguration) -> Option<Vec<String>> {
    if configuration.first_row_as_header {
        table.headers()
    } else {
        None
    }
}

/// Serializes exactly as the document layer does.
fn write(table: &CsvTable, configuration: &CsvConfiguration) -> Result<Vec<u8>, CsvError> {
    let header = header(table, configuration);
    CsvWriter::new(table, header.as_deref(), configuration).data()
}

/// Parses bytes into the table the document would hold.
fn parse(bytes: &[u8], configuration: &CsvConfiguration) -> CsvTable {
    let mut parser =
        CsvParser::from_bytes(bytes, configuration).expect("writer output must decode");
    let rows = parser.read_all().expect("writer output must parse");
    let (headers, body) = if configuration.first_row_as_header {
        let mut rows = rows;
        let headers = if rows.is_empty() {
            None
        } else {
            Some(rows.remove(0))
        };
        (headers, rows)
    } else {
        (None, rows)
    };
    CsvTable::from_rows(body, headers, 1)
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 256,
        ..ProptestConfig::default()
    })]

    /// Quoted output always preserves the table and is byte-stable.
    #[test]
    fn quoted_output_round_trips(table in table_strategy(), case_index in 0..QUOTED_CASES.len()) {
        let case = &QUOTED_CASES[case_index];
        let configuration = configuration(case, table.headers().is_some());
        let bytes = write(&table, &configuration)
            .unwrap_or_else(|error| panic!("{}: quoted write failed: {error:?}", case.name));

        let parsed = parse(&bytes, &configuration);
        prop_assert_eq!(&parsed, &table, "{}: table changed", case.name);
        let rewritten = write(&parsed, &configuration)
            .unwrap_or_else(|error| panic!("{}: quoted rewrite failed: {error:?}", case.name));
        prop_assert_eq!(rewritten, bytes, "{}: bytes changed", case.name);
    }

    /// Unquoted output preserves the table whenever the writer accepts it.
    #[test]
    fn unquoted_output_round_trips_when_accepted(
        table in table_strategy(),
        case_index in 0..UNQUOTED_CASES.len(),
    ) {
        let case = &UNQUOTED_CASES[case_index];
        let configuration = configuration(case, table.headers().is_some());
        let header = header(&table, &configuration);

        match write(&table, &configuration) {
            Ok(bytes) => {
                prop_assert!(
                    !is_ambiguous(&table, header.as_deref(), &configuration),
                    "{}: ambiguous output was written",
                    case.name
                );
                let parsed = parse(&bytes, &configuration);
                prop_assert_eq!(&parsed, &table, "{}: table changed", case.name);
                let rewritten = write(&parsed, &configuration)
                    .unwrap_or_else(|error| panic!("{}: rewrite failed: {error:?}", case.name));
                prop_assert_eq!(rewritten, bytes, "{}: bytes changed", case.name);
            }
            Err(CsvError::AmbiguousOutput) => prop_assert!(
                is_ambiguous(&table, header.as_deref(), &configuration),
                "{}: rejected without an ambiguity",
                case.name
            ),
            Err(error) => prop_assert!(false, "{}: unexpected error {error:?}", case.name),
        }
    }

    /// Arbitrary UTF-8 never panics in any configuration, even when parsing
    /// fails or the separator, quote and escape characters are arbitrary.
    #[test]
    fn arbitrary_utf8_parsing_never_panics(
        input in prop_oneof![
            any::<String>(),
            prop::collection::vec(prop::sample::select(TOKENS), 0..40)
                .prop_map(|tokens| tokens.concat()),
        ],
        separator in config_character(),
        quote in config_character(),
        escape in config_character(),
        first_row_as_header in any::<bool>(),
    ) {
        let configuration = CsvConfiguration {
            column_separator: separator,
            quote_character: quote,
            escape_character: escape,
            first_row_as_header,
            ..CsvConfiguration::default()
        };

        let mut parser = CsvParser::from_string(&input, &configuration);
        let _ = parser.read_all();
        if let Ok(mut parser) = CsvParser::from_bytes(input.as_bytes(), &configuration) {
            let _ = parser.read_all();
        }
    }
}

/// A separator/quote/escape string from ordinary and hostile characters.
fn config_character() -> impl Strategy<Value = String> {
    prop_oneof![
        Just(String::new()),
        prop::sample::select(vec![",", ";", "\t", "é", "\u{0085}", "\u{2028}"])
            .prop_map(str::to_owned),
        any::<char>().prop_map(|character| character.to_string()),
    ]
}

/// The documented normalization: an empty, headerless table cannot be written
/// distinctly from a single empty cell.
#[test]
fn empty_headerless_tables_normalize_to_one_cell() {
    let table = CsvTable::from_rows(Vec::new(), None, 3);
    let configuration = CsvConfiguration::default();

    assert_eq!(table.row_count(), 0);
    assert_eq!(table.columns.len(), 3);

    let bytes = write(&table, &configuration).expect("an empty table writes empty bytes");
    assert!(bytes.is_empty());

    let parsed = parse(&bytes, &configuration);
    assert_eq!(parsed.row_count(), 1);
    assert_eq!(parsed.columns.len(), 1);
    assert_eq!(parsed.cell(0, 0), "");
}
