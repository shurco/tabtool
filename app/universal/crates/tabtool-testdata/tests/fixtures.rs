//! Fixture and generator tests ported from the Apple test suite.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use tabtool_testdata::{
    LargeOptions, fixture, fixture_names, generate_large_csv, make_benchmark_csv, manifest,
    sha256_hex,
};

/// Exact bytes of `makeBenchmarkCSV(rows: 2, cols: 3, quoted: true)` produced
/// by `TestSupport.swift` under Swift 6.4.2.
const BENCH_2_3_TRUE: &str = "col0,col1,col2\n275413.0,\"line one\nline, \"\"two\"\" 0\",892291.2\n763858.0,text764,963250.2\n";

/// SHA-256 and byte count of `generateLargeCSV` output with a 1_000-byte
/// target, 3 columns, a multiline field every 5 rows, CRLF, BOM and seed 1.
/// Computed by running the Apple Swift reference (`LargeFileTestSupport.swift`)
/// under Swift 6.4.2: 31 rows, 1_010 bytes.
const LARGE_GOLDEN_SHA256: &str =
    "bf6e364dfec212b30ec8031fb260f1007ae39a01d3eb07e27f069fdb4b59db24";
const LARGE_GOLDEN_BYTES: u64 = 1_010;
const LARGE_GOLDEN_ROWS: u64 = 31;

fn read(path: &Path) -> Vec<u8> {
    fs::read(path).expect("generated file is readable")
}

fn options(path: &Path, bytes: u64, columns: usize) -> LargeOptions {
    LargeOptions {
        path: path.to_path_buf(),
        bytes,
        columns,
        multiline_every: None,
        crlf: false,
        bom: false,
        seed: 1,
    }
}

#[test]
fn every_manifest_entry_matches_the_bytes() {
    for (name, expected) in manifest() {
        assert_eq!(sha256_hex(&fixture(&name)), expected, "{name}");
    }
}

#[test]
fn copies_the_apple_fixture_counts() {
    assert_eq!(fixture_names().len(), 36);
}

#[test]
fn fixture_names_are_unique_and_match_the_manifest() {
    let names: BTreeSet<String> = fixture_names()
        .iter()
        .map(|name| name.to_string())
        .collect();
    assert_eq!(names.len(), fixture_names().len(), "duplicate fixture name");

    let entries: BTreeSet<String> = manifest().into_keys().collect();
    assert_eq!(entries, names);
}

#[test]
fn sha256_hex_matches_a_known_vector() {
    assert_eq!(
        sha256_hex(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}

#[test]
fn make_benchmark_csv_matches_the_apple_bytes() {
    let csv = make_benchmark_csv(2, 3, true);
    assert_eq!(csv, BENCH_2_3_TRUE);
    assert_eq!(csv.len(), 86);
    assert_eq!(
        sha256_hex(csv.as_bytes()),
        "93e94fc4fab1df7dbff84f3ff0363541a498eaa0a370297e011f5647ef114cb2"
    );
}

#[test]
fn make_benchmark_csv_is_deterministic() {
    for quoted in [true, false] {
        let first = make_benchmark_csv(50, 5, quoted);
        let second = make_benchmark_csv(50, 5, quoted);
        assert_eq!(first, second, "quoted={quoted}");
    }
}

#[test]
fn large_same_seed_gives_identical_bytes() {
    let dir = tempfile::tempdir().expect("temp directory");
    let mut same = options(&dir.path().join("a.csv"), 1 << 20, 5);
    same.multiline_every = Some(7);
    let mut other = same.clone();
    other.path = dir.path().join("b.csv");
    let mut different = same.clone();
    different.path = dir.path().join("c.csv");
    different.seed = 2;

    let a = generate_large_csv(&same).expect("first file");
    let b = generate_large_csv(&other).expect("second file");
    let c = generate_large_csv(&different).expect("third file");

    assert_eq!(read(&a.path), read(&b.path));
    assert_ne!(read(&a.path), read(&c.path));
    assert!(a.bytes >= 1 << 20);
    assert_eq!(a.bytes, fs::metadata(&a.path).expect("metadata").len());
}

#[test]
fn large_crlf_has_no_bare_line_feed_outside_multiline_fields() {
    let dir = tempfile::tempdir().expect("temp directory");

    let mut plain = options(&dir.path().join("plain.csv"), 1 << 20, 5);
    plain.crlf = true;
    let plain = generate_large_csv(&plain).expect("plain file");
    let plain_bytes = read(&plain.path);
    assert_eq!(
        plain_bytes.iter().filter(|&&byte| byte == b'\n').count(),
        plain.rows as usize
    );
    assert_eq!(
        plain_bytes.iter().filter(|&&byte| byte == b'\r').count(),
        plain.rows as usize
    );

    let mut multiline = options(&dir.path().join("multiline.csv"), 1 << 20, 5);
    multiline.crlf = true;
    multiline.multiline_every = Some(10);
    let multiline = generate_large_csv(&multiline).expect("multiline file");
    let bytes = read(&multiline.path);
    let bare = bytes
        .iter()
        .enumerate()
        .filter(|&(index, &byte)| byte == b'\n' && (index == 0 || bytes[index - 1] != b'\r'))
        .count();
    assert_eq!(bare, (multiline.rows as usize - 1) / 10);
}

#[test]
fn large_reports_row_and_column_counts() {
    let dir = tempfile::tempdir().expect("temp directory");
    let fixture =
        generate_large_csv(&options(&dir.path().join("plain.csv"), 500, 2)).expect("plain file");
    let bytes = read(&fixture.path);

    assert_eq!(
        fixture.rows as usize,
        bytes.iter().filter(|&&byte| byte == b'\n').count()
    );
    assert_eq!(fixture.bytes, bytes.len() as u64);
    assert!(fixture.bytes >= 500);

    let text = String::from_utf8(bytes).expect("generated text is UTF-8");
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), fixture.rows as usize);
    assert_eq!(lines.first(), Some(&"col0,col1"));
    for line in lines {
        assert_eq!(line.split(',').count(), 2, "{line}");
    }
}

#[test]
fn large_bom_prefix_is_written() {
    let dir = tempfile::tempdir().expect("temp directory");
    let mut bom = options(&dir.path().join("bom.csv"), 1_024, 5);
    bom.bom = true;
    let fixture = generate_large_csv(&bom).expect("bom file");
    let bytes = read(&fixture.path);

    assert_eq!(&bytes[..3], &[0xEF, 0xBB, 0xBF]);
    assert_eq!(fixture.bytes, bytes.len() as u64);
    assert_eq!(
        bytes.iter().filter(|&&byte| byte == b'\n').count(),
        fixture.rows as usize
    );
}

#[test]
fn large_matches_the_apple_bytes() {
    let dir = tempfile::tempdir().expect("temp directory");
    let mut golden = options(&dir.path().join("golden.csv"), 1_000, 3);
    golden.multiline_every = Some(5);
    golden.crlf = true;
    golden.bom = true;
    golden.seed = 1;

    let fixture = generate_large_csv(&golden).expect("golden file");
    let bytes = read(&fixture.path);

    assert_eq!(fixture.bytes, LARGE_GOLDEN_BYTES);
    assert_eq!(fixture.rows, LARGE_GOLDEN_ROWS);
    assert_eq!(bytes.len() as u64, LARGE_GOLDEN_BYTES);
    assert_eq!(sha256_hex(&bytes), LARGE_GOLDEN_SHA256);
}
