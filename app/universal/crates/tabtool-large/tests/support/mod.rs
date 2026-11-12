//! Shared helpers for the large-file tests.

#![allow(dead_code)] // Each test crate uses a different subset of these helpers.

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use tabtool_core::{CsvConfiguration, CsvParser};
use tabtool_large::{LargeFileIndexer, MappedFile, RowIndex, RowScanner, ScanMode, row_ranges};

/// SplitMix64, matching `SeededGenerator` in `TestSupport.swift`.
///
/// The bit stream matches Swift, but [`SplitMix64::below`] reduces with modulo
/// while Swift's `randomElement`/`Int.random` use rejection sampling. The
/// generated corpus is deterministic but not byte-identical to Swift's fuzz
/// corpus; [`fixed_corpus`] anchors the boundary coverage independently of the
/// generator.
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

/// Fixed byte inputs around the 64-byte block edge, independent of the RNG.
pub fn fixed_corpus() -> Vec<Vec<u8>> {
    vec![
        b"".to_vec(),
        b"a".to_vec(),
        b"\r\n".to_vec(),
        b"a\nb\r\nc\rd".to_vec(),
        b"\"x\ny\",z\r\nq".to_vec(),
        [vec![b'a'; 64], b"\r\n".to_vec(), vec![b'b'; 64]].concat(),
        [
            vec![b'a'; 63],
            b",\"a\"\"b\",c\r\n".to_vec(),
            vec![b'd'; 65],
        ]
        .concat(),
        [
            vec![b'a'; 64],
            b"\r".to_vec(),
            b"\"\r\n\"".to_vec(),
            b"\n".to_vec(),
        ]
        .concat(),
    ]
}

/// Asserts that `row_ranges` covers `bytes` contiguously with the same rows as
/// `CsvParser`, and that every non-final range ends with exactly one
/// terminator.
pub fn expect_ranges_match_rows(bytes: &[u8], configuration: &CsvConfiguration) {
    let ranges = row_ranges(bytes, configuration);
    let index = scan_index(bytes, configuration);
    let mut parser =
        CsvParser::from_bytes(bytes, configuration).expect("test input decodes with the encoding");
    let rows = parser.read_all().expect("test input is valid CSV");
    assert_eq!(ranges.len(), index.row_count);
    assert_eq!(ranges.len(), rows.len());
    let mut start = 0;
    for (row, range) in ranges.iter().enumerate() {
        assert_eq!(range.start, start, "row {row} starts where the last ended");
        assert!(range.start <= range.end && range.end <= bytes.len());
        start = range.end;
        let slice = &bytes[range.clone()];
        let terminator = if slice.ends_with(b"\r\n") {
            2
        } else if slice.ends_with(b"\n") || slice.ends_with(b"\r") {
            1
        } else {
            0
        };
        if row + 1 < ranges.len() {
            assert!(terminator > 0, "non-final row {row} has no terminator");
        }
        if terminator > 0 {
            let content = &slice[..slice.len() - terminator];
            assert!(
                !content.ends_with(b"\n") && !content.ends_with(b"\r"),
                "row {row} has more than one terminator"
            );
        }
        let mut content = CsvParser::from_bytes(slice, configuration).expect("row content decodes");
        assert_eq!(
            content.read_all().expect("row content parses"),
            vec![rows[row].clone()],
            "row {row}"
        );
    }
    assert_eq!(start, bytes.len());
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

/// Writes a deterministic quoted CSV of at least `target_bytes` to `path` and
/// returns the row count, header included.
///
/// Even columns hold numbers, odd columns quoted text, and every seventh row
/// carries a multiline quoted field with embedded separators and quotes. Ported
/// from `generateLargeCSV` in `LargeFileTestSupport.swift` with the indexer
/// fixture's settings.
fn generate_large_csv(path: &Path, target_bytes: usize, columns: usize, seed: u64) -> usize {
    let mut writer = BufWriter::new(File::create(path).expect("fixture file is created"));
    let mut generator = SplitMix64::new(seed);
    let header = (0..columns)
        .map(|column| format!("col{column}"))
        .collect::<Vec<_>>()
        .join(",");
    writeln!(writer, "{header}").expect("header is written");
    let mut written = header.len() + 1;
    let mut rows = 1;
    while written < target_bytes {
        let mut line = String::new();
        for column in 0..columns {
            if column > 0 {
                line.push(',');
            }
            let value = generator.next();
            if rows % 7 == 0 && column == 1.min(columns - 1) {
                line.push_str(&format!("\"line one\nline, \"\"two\"\" {rows}\""));
            } else if column % 2 == 0 {
                line.push_str(&format!("{}.{column}", value % 1_000_000));
            } else {
                line.push_str(&format!("\"text{}\"", value % 5000));
            }
        }
        line.push('\n');
        written += line.len();
        writer.write_all(line.as_bytes()).expect("row is written");
        rows += 1;
    }
    writer.flush().expect("fixture file flushes");
    rows
}

/// A mapped large CSV fixture kept alive for the test, with the generator's
/// row count. Matches `Fixture` in `LargeFileIndexerTests.swift`.
pub struct LargeCsvFixture {
    _directory: tempfile::TempDir,
    pub file: Arc<MappedFile>,
    pub rows: usize,
}

impl LargeCsvFixture {
    /// The 2 MiB fixture used by the ported indexer tests.
    pub fn new() -> Self {
        Self::with_target(2 * 1_048_576)
    }

    /// A fixture of at least `target_bytes`.
    pub fn with_target(target_bytes: usize) -> Self {
        let directory = tempfile::tempdir().expect("fixture directory is created");
        let path = directory.path().join("big.csv");
        let rows = generate_large_csv(&path, target_bytes, 5, 3);
        let file = Arc::new(MappedFile::open(&path).expect("fixture file opens"));
        Self {
            _directory: directory,
            file,
            rows,
        }
    }

    /// The path of the fixture file.
    pub fn path(&self) -> PathBuf {
        self._directory.path().join("big.csv")
    }
}

/// An indexer over `fixture` with the default row rules, not started yet.
pub fn large_file_indexer(
    fixture: &LargeCsvFixture,
    step_bytes: usize,
    progress_interval: Duration,
) -> Arc<LargeFileIndexer> {
    Arc::new(LargeFileIndexer::new(
        Arc::clone(&fixture.file),
        CsvConfiguration::default(),
        step_bytes,
        progress_interval,
    ))
}
