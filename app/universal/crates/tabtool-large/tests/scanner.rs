//! Ported from `RowScannerTests.swift` and the `RowIndex` cases used by
//! `LargeFileIndexerTests.swift`, plus chunk-boundary and row-range cases.

mod support;

use std::ops::Range;

use support::{SplitMix64, expect_index_matches_parser, make_input, scan_index, scan_with};
use tabtool_core::{CsvConfiguration, CsvParser};
use tabtool_large::{RowIndex, RowScanner, ScanMode, row_ranges};

const EDGE_CASES: &[&str] = &[
    "",
    "a",
    "a\n",
    "a\n\n",
    "\n",
    "a\r\nb",
    "a\rb\r",
    "a\r\n",
    "\r\n\r\n",
    "\r\r\n\n",
    "a,b,\n",
    ",",
    "\"x\ny\",z\nq",
    "\"a\"\"b\"\n1",
    "\"\"",
    "a,\"b\r\nc\"\r\nd",
    "\n\n\n",
    "x\r\ny\rz\nw",
];

fn configuration() -> CsvConfiguration {
    CsvConfiguration::default()
}

fn escape_configuration() -> CsvConfiguration {
    CsvConfiguration {
        escape_character: "\\".to_string(),
        ..CsvConfiguration::default()
    }
}

fn row_contents<'a>(bytes: &'a [u8], ranges: &[Range<usize>]) -> Vec<&'a [u8]> {
    ranges.iter().map(|range| &bytes[range.clone()]).collect()
}

fn expect_ranges(text: &str, configuration: &CsvConfiguration, expected: &[&str]) {
    let bytes = text.as_bytes();
    let ranges = row_ranges(bytes, configuration);
    let contents = row_contents(bytes, &ranges);
    let expected: Vec<&[u8]> = expected.iter().map(|text| text.as_bytes()).collect();
    assert_eq!(contents, expected, "row ranges of {text:?}");
}

#[test]
fn matches_parser_on_edge_cases() {
    for text in EDGE_CASES {
        let bytes = text.as_bytes();
        let index = scan_index(bytes, &configuration());
        expect_index_matches_parser(bytes, &index, &configuration());
    }
}

#[test]
fn resumes_across_chunks() {
    for text in EDGE_CASES {
        let bytes = text.as_bytes();
        let whole = scan_index(bytes, &configuration());
        for cut in 0..=bytes.len() {
            let chunked = scan_with(bytes, &configuration(), ScanMode::Automatic, &[cut]);
            assert_eq!(chunked, whole, "cut {cut} of {text:?}");
        }
        let every_byte: Vec<usize> = (0..bytes.len()).collect();
        let chunked = scan_with(bytes, &configuration(), ScanMode::Automatic, &every_byte);
        assert_eq!(chunked, whole, "one-byte chunks of {text:?}");
    }
}

#[test]
fn checkpoints_at_multiples_of_64() {
    let mut text = String::new();
    for row in 0..200 {
        text.push_str(&format!("r{row}\n"));
    }
    let bytes = text.as_bytes();
    let index = scan_index(bytes, &configuration());
    assert_eq!(index.row_count, 200);
    assert_eq!(index.checkpoints.len(), 4);
    assert_eq!(index.block_range(0, bytes.len()).unwrap().start, 0);
    assert_eq!(index.block_range(4, bytes.len()), None);
    let last = index.block_range(3, bytes.len()).expect("last block");
    assert_eq!(last.end, bytes.len());
    expect_index_matches_parser(bytes, &index, &configuration());
}

#[test]
fn no_checkpoint_for_row_that_never_starts() {
    let mut text = String::new();
    for row in 0..64 {
        text.push_str(&format!("r{row}\n"));
    }
    let index = scan_index(text.as_bytes(), &configuration());
    assert_eq!(index.row_count, 64);
    assert_eq!(index.checkpoints.len(), 1);
}

#[test]
fn checkpoint_on_the_64th_row_boundary() {
    let mut first_64 = String::new();
    for row in 0..64 {
        first_64.push_str(&format!("r{row}\n"));
    }
    let boundary = first_64.len();
    let mut text = first_64;
    text.push_str("r64\n");
    let index = scan_index(text.as_bytes(), &configuration());
    assert_eq!(index.row_count, 65);
    assert_eq!(index.checkpoints, vec![0, boundary as u64]);
}

#[test]
fn backslash_escape_keeps_escaped_quote() {
    let configuration = escape_configuration();
    for text in ["\"a\\\"b\",c\nd", "\"a\\\\\",x\ny", "\"a\\\n\",x\ny"] {
        let bytes = text.as_bytes();
        let index = scan_index(bytes, &configuration);
        expect_index_matches_parser(bytes, &index, &configuration);
        assert_eq!(index.row_count, 2, "{text:?}");
    }
}

#[test]
fn quoteless_configuration_ignores_quotes() {
    let configuration = CsvConfiguration {
        quote_character: String::new(),
        ..CsvConfiguration::default()
    };
    let index = scan_index(b"a\"b\nc", &configuration);
    assert_eq!(index.row_count, 2);
    assert!(!index.ends_inside_quote);
}

#[test]
fn tab_separator_counts_fields() {
    let configuration = CsvConfiguration {
        column_separator: "\t".to_string(),
        ..CsvConfiguration::default()
    };
    let index = scan_index(b"a\tb\tc\nd", &configuration);
    assert_eq!(index.max_field_count, 3);
}

#[test]
fn unbalanced_quote_sets_flag() {
    let index = scan_index(b"a\n\"b\nc\n", &configuration());
    assert!(index.ends_inside_quote);
    assert_eq!(index.row_count, 2);
}

#[test]
fn fuzz_matches_parser() {
    for seed in 1..=200u64 {
        let bytes = make_input(seed, 150);
        let whole = scan_index(&bytes, &configuration());
        expect_index_matches_parser(&bytes, &whole, &configuration());
        let mut generator = SplitMix64::new(seed);
        let mut cuts = (0..3)
            .map(|_| generator.below(bytes.len() + 1))
            .collect::<Vec<_>>();
        cuts.sort_unstable();
        let chunked = scan_with(&bytes, &configuration(), ScanMode::Automatic, &cuts);
        assert_eq!(chunked, whole, "seed {seed} cuts {cuts:?}");
    }
}

#[test]
fn crlf_split_across_chunks() {
    let bytes = b"a\r\nb";
    let mut scanner = RowScanner::new(&configuration());
    let mut index = RowIndex::new();
    scanner.scan(&bytes[..2], 0, &mut index, false);
    assert_eq!(index.row_count, 1);
    assert_eq!(index.scanned_bytes, 2);
    assert!(!index.ends_inside_quote);
    scanner.scan(&bytes[2..], 2, &mut index, true);
    assert_eq!(index, scan_index(bytes, &configuration()));
    assert_eq!(index.row_count, 2);
}

#[test]
fn quote_split_across_chunks() {
    let bytes = b"\"x\ny\",z\nq";
    let mut scanner = RowScanner::new(&configuration());
    let mut index = RowIndex::new();
    scanner.scan(&bytes[..1], 0, &mut index, false);
    assert!(index.ends_inside_quote);
    assert_eq!(index.row_count, 1);
    scanner.scan(&bytes[1..], 1, &mut index, true);
    assert_eq!(index, scan_index(bytes, &configuration()));
    assert_eq!(index.row_count, 2);
    assert!(!index.ends_inside_quote);
}

#[test]
fn doubled_quote_split_across_chunks() {
    let bytes = b"\"a\"\"b\"\n1";
    let mut scanner = RowScanner::new(&configuration());
    let mut index = RowIndex::new();
    scanner.scan(&bytes[..3], 0, &mut index, false);
    scanner.scan(&bytes[3..], 3, &mut index, true);
    assert_eq!(index, scan_index(bytes, &configuration()));
    assert_eq!(index.row_count, 2);
}

#[test]
fn escaped_quote_at_the_final_byte() {
    let configuration = escape_configuration();
    for text in ["\"a\\\"", "\"a\\", "\"a\\\\\",x\ny"] {
        let bytes = text.as_bytes();
        let whole = scan_index(bytes, &configuration);
        for cut in 0..=bytes.len() {
            let chunked = scan_with(bytes, &configuration, ScanMode::Automatic, &[cut]);
            assert_eq!(chunked, whole, "cut {cut} of {text:?}");
        }
    }
}

#[test]
fn final_chunk_without_terminator() {
    let bytes = b"a\nb";
    let index = scan_index(bytes, &configuration());
    assert_eq!(index.row_count, 2);
    assert_eq!(index.max_field_count, 1);
    assert_eq!(index.scanned_bytes, 3);
}

#[test]
fn empty_input_is_one_row() {
    let mut scanner = RowScanner::new(&configuration());
    let mut index = RowIndex::new();
    scanner.scan(&[], 0, &mut index, true);
    assert_eq!(index.row_count, 1);
    assert_eq!(index.max_field_count, 1);
    assert_eq!(index.checkpoints, vec![0]);
    assert_eq!(index.scanned_bytes, 0);
    assert!(!index.ends_inside_quote);
    expect_index_matches_parser(b"", &index, &configuration());
}

#[test]
fn extend_appends_checkpoints_and_takes_over_counters() {
    let mut text = String::new();
    for row in 0..200 {
        text.push_str(&format!("r{row}\n"));
    }
    let bytes = text.as_bytes();
    let mut published = RowIndex::new();
    let mut scanner = RowScanner::new(&configuration());
    let mut local = RowIndex::new();
    let step = 333;
    let mut offset = 0;
    while offset < bytes.len() {
        let end = (offset + step).min(bytes.len());
        scanner.scan(&bytes[offset..end], offset, &mut local, end == bytes.len());
        published.extend(local.clone());
        local.checkpoints.clear();
        offset = end;
    }
    assert_eq!(published, scan_index(bytes, &configuration()));
    assert_eq!(published.row_count, 200);
    assert_eq!(published.checkpoints.len(), 4);
}

#[test]
fn block_range_cases() {
    let index = RowIndex {
        checkpoints: vec![0, 10, 20],
        row_count: 192,
        max_field_count: 1,
        scanned_bytes: 30,
        ends_inside_quote: false,
    };
    assert_eq!(index.block_range(0, 30), Some(0..10));
    assert_eq!(index.block_range(1, 30), Some(10..20));
    assert_eq!(index.block_range(2, 30), Some(20..30));
    assert_eq!(index.block_range(3, 30), None);
    assert_eq!(index.block_range(0, 5), Some(0..10));
    assert_eq!(index.block_range(2, 5), Some(20..20));
}

#[test]
fn row_ranges_exclude_terminators() {
    let configuration = configuration();
    let cases: &[(&str, &[&str])] = &[
        ("", &[""]),
        ("a", &["a"]),
        ("a\n", &["a"]),
        ("a\nb", &["a", "b"]),
        ("a\r\nb", &["a", "b"]),
        ("a\rb", &["a", "b"]),
        ("\n", &[""]),
        ("\n\n", &["", ""]),
        ("\r\n\r\n", &["", ""]),
        ("a\n\nb", &["a", "", "b"]),
        ("\"x\ny\",z\nq", &["\"x\ny\",z", "q"]),
        ("a,\"b\r\nc\"\r\nd", &["a,\"b\r\nc\"", "d"]),
        ("\"a\"\"b\"\n1", &["\"a\"\"b\"", "1"]),
        ("\"\"\n", &["\"\""]),
        ("a\r", &["a"]),
        ("a\r\n", &["a"]),
        ("x\r\ny\rz\nw", &["x", "y", "z", "w"]),
        ("\"x\ny\"", &["\"x\ny\""]),
    ];
    for (text, expected) in cases {
        expect_ranges(text, &configuration, expected);
    }
}

#[test]
fn row_ranges_follow_escape_and_quoteless_configurations() {
    let escape = escape_configuration();
    expect_ranges("\"a\\\"b\",c\nd", &escape, &["\"a\\\"b\",c", "d"]);
    expect_ranges("\"a\\\\\",x\ny", &escape, &["\"a\\\\\",x", "y"]);
    expect_ranges("\"a\\\n\",x\ny", &escape, &["\"a\\\n\",x", "y"]);

    let quoteless = CsvConfiguration {
        quote_character: String::new(),
        ..CsvConfiguration::default()
    };
    expect_ranges("a\"b\nc", &quoteless, &["a\"b", "c"]);
}

#[test]
fn row_ranges_leave_one_terminator_between_rows() {
    let configuration = configuration();
    for text in EDGE_CASES {
        let bytes = text.as_bytes();
        let ranges = row_ranges(bytes, &configuration);
        let index = scan_index(bytes, &configuration);
        assert_eq!(ranges.len(), index.row_count, "row count of {text:?}");
        for (row, range) in ranges.iter().enumerate() {
            assert!(range.start <= range.end, "row {row} of {text:?}");
            assert!(range.end <= bytes.len(), "row {row} of {text:?}");
            if row > 0 {
                assert!(
                    ranges[row - 1].end <= range.start,
                    "rows overlap in {text:?}"
                );
            }
            let next = ranges.get(row + 1).map_or(bytes.len(), |next| next.start);
            let gap = &bytes[range.end..next];
            assert!(
                matches!(gap, b"" | b"\n" | b"\r" | b"\r\n"),
                "gap {gap:?} after row {row} of {text:?}"
            );
        }
    }
}

#[test]
fn row_ranges_match_rows_on_fuzz() {
    let configuration = configuration();
    for seed in 1..=100u64 {
        let bytes = make_input(seed, 150);
        let ranges = row_ranges(&bytes, &configuration);
        let index = scan_index(&bytes, &configuration);
        let mut parser = CsvParser::from_bytes(&bytes, &configuration).expect("fuzz input decodes");
        let rows = parser.read_all().expect("fuzz input parses");
        assert_eq!(ranges.len(), index.row_count, "seed {seed}");
        assert_eq!(ranges.len(), rows.len(), "seed {seed}");
        for (row, range) in ranges.iter().enumerate() {
            assert!(range.start <= range.end && range.end <= bytes.len());
            if row > 0 {
                assert!(
                    ranges[row - 1].end <= range.start,
                    "rows overlap at {row} of seed {seed}"
                );
            }
            let mut content = CsvParser::from_bytes(&bytes[range.clone()], &configuration)
                .expect("row content decodes");
            assert_eq!(
                content.read_all().expect("row content parses"),
                vec![rows[row].clone()],
                "row {row} of seed {seed}"
            );
            let next = ranges.get(row + 1).map_or(bytes.len(), |next| next.start);
            let gap = &bytes[range.end..next];
            assert!(
                matches!(gap, b"" | b"\n" | b"\r" | b"\r\n"),
                "gap {gap:?} after row {row} of seed {seed}"
            );
        }
    }
}
