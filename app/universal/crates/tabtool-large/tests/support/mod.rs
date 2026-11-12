//! Shared helpers for the scanner tests.

#![allow(dead_code)] // Each test crate uses a different subset of these helpers.

use tabtool_core::{CsvConfiguration, CsvParser};
use tabtool_large::{RowIndex, RowScanner, ScanMode};

/// SplitMix64, matching `SeededGenerator` in `TestSupport.swift`.
pub struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    pub fn next(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut value = self.state;
        value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        value ^ (value >> 31)
    }

    pub fn below(&mut self, bound: usize) -> usize {
        (self.next() % bound as u64) as usize
    }
}

/// Scans `bytes` in chunks cut at the given increasing offsets and returns the
/// final index. The last chunk is scanned with `final_chunk`.
pub fn scan_with(
    bytes: &[u8],
    configuration: &CsvConfiguration,
    mode: ScanMode,
    cuts: &[usize],
) -> RowIndex {
    let mut scanner = RowScanner::new(configuration);
    scanner.set_mode(mode);
    let mut index = RowIndex::new();
    let mut start = 0;
    for &cut in cuts {
        assert!(start <= cut && cut <= bytes.len(), "cut is in bounds");
        scanner.scan(&bytes[start..cut], start, &mut index, false);
        start = cut;
    }
    scanner.scan(&bytes[start..], start, &mut index, true);
    index
}

/// Scans `bytes` in one automatic-mode chunk.
pub fn scan_index(bytes: &[u8], configuration: &CsvConfiguration) -> RowIndex {
    scan_with(bytes, configuration, ScanMode::Automatic, &[])
}

/// Asserts that `index` agrees with `CsvParser`, including every checkpoint's
/// first row. Ported from `expectIndexMatchesParser` in `RowScannerTests.swift`.
pub fn expect_index_matches_parser(
    bytes: &[u8],
    index: &RowIndex,
    configuration: &CsvConfiguration,
) {
    let mut parser =
        CsvParser::from_bytes(bytes, configuration).expect("test input decodes with the encoding");
    let rows = parser.read_all().expect("test input is valid CSV");
    assert_eq!(index.row_count, rows.len());
    assert_eq!(
        index.max_field_count,
        rows.iter().map(Vec::len).max().unwrap_or(0)
    );
    assert_eq!(index.checkpoints.len(), rows.len().div_ceil(64));
    for (block, &offset) in index.checkpoints.iter().enumerate() {
        let offset = usize::try_from(offset).expect("checkpoint fits usize");
        let mut tail = CsvParser::from_bytes(&bytes[offset..], configuration)
            .expect("checkpoint tail decodes");
        let row = tail
            .next_row()
            .expect("checkpoint tail parses")
            .expect("checkpoint points at a row");
        assert_eq!(row, rows[block * 64], "checkpoint {block} row");
    }
}

/// Random CSV bytes with mixed terminators, quoting and multibyte text.
/// Ported from `RowScannerFuzz.makeInput`.
pub fn make_input(seed: u64, rows: usize) -> Vec<u8> {
    const PLAIN: &[&str] = &["a", "bb", "", "é", "日本", "12.5", "x y"];
    const SPECIAL: &[&str] = &[",", "\n", "\r\n", "\r", "\"", "\"\"", "q,\n\"z"];
    const TERMINATORS: &[&str] = &["\n", "\r\n", "\r"];

    let mut generator = SplitMix64::new(seed);
    let mut text = String::new();
    for row in 0..rows {
        let field_count = generator.below(5) + 1;
        let mut fields = Vec::with_capacity(field_count);
        for _ in 0..field_count {
            let value = PLAIN[generator.below(PLAIN.len())];
            if generator.next().is_multiple_of(4) {
                let extra = SPECIAL[generator.below(SPECIAL.len())];
                let quoted = format!("{value}{extra}").replace('"', "\"\"");
                fields.push(format!("\"{quoted}\""));
            } else {
                fields.push(value.to_string());
            }
        }
        text.push_str(&fields.join(","));
        if row < rows - 1 || generator.next().is_multiple_of(2) {
            text.push_str(TERMINATORS[generator.below(TERMINATORS.len())]);
        }
    }
    text.into_bytes()
}
