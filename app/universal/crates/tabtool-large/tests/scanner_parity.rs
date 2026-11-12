//! Ported from `RowScannerSIMDTests.swift`: the block path must agree with the
//! scalar path on every boundary and keep the throughput shape.

mod support;

use std::time::Instant;

use support::{expect_index_matches_parser, make_input, scan_with};
use tabtool_core::CsvConfiguration;
use tabtool_large::{RowScanner, ScanMode};

fn configuration() -> CsvConfiguration {
    CsvConfiguration::default()
}

fn escape_configuration() -> CsvConfiguration {
    CsvConfiguration {
        escape_character: "\\".to_string(),
        ..CsvConfiguration::default()
    }
}

#[test]
fn block_matches_scalar_on_fuzz() {
    for seed in 1..=200u64 {
        let bytes = make_input(seed, 400);
        let scalar = scan_with(&bytes, &configuration(), ScanMode::Scalar, &[]);
        assert_eq!(
            scan_with(&bytes, &configuration(), ScanMode::Block, &[]),
            scalar,
            "seed {seed}"
        );
        let cuts: Vec<usize> = [37, 64, 65, 200, 1001]
            .into_iter()
            .filter(|&cut| cut <= bytes.len())
            .collect();
        assert_eq!(
            scan_with(&bytes, &configuration(), ScanMode::Automatic, &cuts),
            scalar,
            "seed {seed} with cuts"
        );
        expect_index_matches_parser(&bytes, &scalar, &configuration());
    }
}

#[test]
fn block_handles_boundary_cases() {
    let patterns = [
        "\r\n", "\r", "\n", "\"", "\"\"", "\"x\ny\"", ",", "\"\r\n\"", "\r\r\n",
    ];
    let positions = [0, 62, 63, 64, 65, 126, 127, 128];
    for pattern in patterns {
        for length in 0..=130 {
            for &position in positions.iter().filter(|&&position| position <= length) {
                let mut text = "a".repeat(position);
                text.push_str(pattern);
                text.push_str(&"b".repeat(length - position));
                let bytes = text.as_bytes();
                let scalar = scan_with(bytes, &configuration(), ScanMode::Scalar, &[]);
                assert_eq!(
                    scan_with(bytes, &configuration(), ScanMode::Block, &[]),
                    scalar,
                    "{pattern:?} at {position} of {length}"
                );
            }
        }
    }
}

#[test]
fn every_length_matches_scalar() {
    let unit = b"ab,\"c\r\nd\"\r\n\n";
    let bytes: Vec<u8> = (0..20).flat_map(|_| unit.iter().copied()).collect();
    for length in 0..=bytes.len() {
        let slice = &bytes[..length];
        assert_eq!(
            scan_with(slice, &configuration(), ScanMode::Block, &[]),
            scan_with(slice, &configuration(), ScanMode::Scalar, &[]),
            "length {length}"
        );
    }
}

#[test]
fn backslash_config_uses_scalar_path() {
    let configuration = escape_configuration();
    let mut text = "x".repeat(62);
    text.push_str("\"a\\\"b\",c\nd");
    text.push_str(&"y".repeat(80));
    let bytes = text.as_bytes();
    let forced = scan_with(bytes, &configuration, ScanMode::Block, &[]);
    assert_eq!(
        forced,
        scan_with(bytes, &configuration, ScanMode::Scalar, &[])
    );
    assert_eq!(forced.row_count, 2);
}

#[test]
fn quoteless_config_matches_scalar() {
    let configuration = CsvConfiguration {
        quote_character: String::new(),
        ..CsvConfiguration::default()
    };
    let bytes = make_input(7, 300);
    assert_eq!(
        scan_with(&bytes, &configuration, ScanMode::Block, &[]),
        scan_with(&bytes, &configuration, ScanMode::Scalar, &[])
    );
}

#[test]
fn block_scan_throughput() {
    let unit = b"12345.5,text value,\"quoted, field\",abc\r\n";
    let size = if cfg!(debug_assertions) {
        4 << 20
    } else {
        64 << 20
    };
    let rows = size / unit.len();
    let bytes: Vec<u8> = unit
        .iter()
        .copied()
        .cycle()
        .take(rows * unit.len())
        .collect();
    let mut scanner = RowScanner::new(&configuration());
    scanner.set_mode(ScanMode::Block);
    let mut index = tabtool_large::RowIndex::new();
    let start = Instant::now();
    scanner.scan(&bytes, 0, &mut index, true);
    let seconds = start.elapsed().as_secs_f64();
    assert_eq!(index.row_count, rows);
    let rate = bytes.len() as f64 / seconds / 1e9;
    println!("block scan throughput: {rate:.2} GB/s");
    #[cfg(not(debug_assertions))]
    assert!(rate >= 0.25, "block scan throughput was {rate:.2} GB/s");
}
