//! Shared helpers for the large-file tests.

#![allow(dead_code)] // Each test crate uses a different subset of these helpers.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use tabtool_core::{CsvConfiguration, CsvParser};
use tabtool_large::{LargeFileIndexer, MappedFile, RowIndex, RowScanner, ScanMode, row_ranges};
use tabtool_testdata::{LargeOptions, SplitMix64, generate_large_csv};

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
///
/// [`SplitMix64::below`] reduces with modulo, so the generated corpus is
/// deterministic but not byte-identical to Swift's fuzz corpus;
/// [`fixed_corpus`] anchors the boundary coverage independently of the
/// generator.
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
        let rows = large_csv(&path, target_bytes, 5, true, Some(7), 3);
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

/// Writes the `tabtool_testdata` large CSV (LF rows, no BOM) to `path` and
/// returns its row count, header included.
pub fn large_csv(
    path: &Path,
    target_bytes: usize,
    columns: usize,
    quoted: bool,
    multiline_every: Option<usize>,
    seed: u64,
) -> usize {
    let fixture = generate_large_csv(&LargeOptions {
        path: path.to_path_buf(),
        bytes: target_bytes as u64,
        columns,
        multiline_every,
        crlf: false,
        bom: false,
        quoted,
        seed,
    })
    .expect("fixture file is written");
    usize::try_from(fixture.rows).expect("row count fits usize")
}

/// A scratch directory removed when the value goes away.
pub struct TempDirectory {
    directory: tempfile::TempDir,
}

impl TempDirectory {
    pub fn new() -> Self {
        Self {
            directory: tempfile::tempdir().expect("temporary directory is created"),
        }
    }

    /// The directory path.
    pub fn path(&self) -> &Path {
        self.directory.path()
    }

    /// Writes `bytes` to `name` in the directory and returns its path.
    pub fn file(&self, name: &str, bytes: &[u8]) -> PathBuf {
        let path = self.path().join(name);
        std::fs::write(&path, bytes).expect("fixture file is written");
        path
    }
}

impl Default for TempDirectory {
    fn default() -> Self {
        Self::new()
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
