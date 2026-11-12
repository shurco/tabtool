//! Ported from `LargeCSVSourceTests.swift`, plus the block-boundary and
//! snapshot cases the grid and the saver rely on.

mod support;

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use support::{TempDirectory, generate_large_csv};
use tabtool_core::{CsvConfiguration, CsvError, CsvParser, CsvTable, Encoding, RowSource};
#[cfg(unix)]
use tabtool_large::replace_file;
use tabtool_large::{
    CellKey, CellOverlay, LargeCSVSource, LargeFile, LargeFileIndexer, MappedFile, Staleness,
};

const STEP_BYTES: usize = 4096;

fn configuration() -> CsvConfiguration {
    CsvConfiguration::default()
}

fn make_indexer(
    file: Arc<MappedFile>,
    configuration: &CsvConfiguration,
    step_bytes: usize,
) -> Arc<LargeFileIndexer> {
    Arc::new(LargeFileIndexer::new(
        file,
        configuration.clone(),
        step_bytes,
        Duration::ZERO,
    ))
}

fn source_with(
    file: Arc<MappedFile>,
    configuration: &CsvConfiguration,
    indexer: Arc<LargeFileIndexer>,
    overlay: CellOverlay,
    column_ids: Vec<tabtool_core::ColumnId>,
) -> LargeCSVSource {
    LargeCSVSource::new(file, configuration.clone(), indexer, overlay, column_ids)
}

fn make_source(file: Arc<MappedFile>, configuration: &CsvConfiguration) -> LargeCSVSource {
    let indexer = make_indexer(Arc::clone(&file), configuration, STEP_BYTES);
    indexer.start(|_| {});
    indexer.wait();
    source_with(file, configuration, indexer, CellOverlay::new(), Vec::new())
}

/// The source over a fully scanned file, stopped after its first step.
fn partial_source(
    file: Arc<MappedFile>,
    configuration: &CsvConfiguration,
    step_bytes: usize,
) -> LargeCSVSource {
    let indexer = make_indexer(Arc::clone(&file), configuration, step_bytes);
    let scan = Arc::clone(&indexer);
    indexer.start(move |_| scan.cancel());
    indexer.wait();
    source_with(file, configuration, indexer, CellOverlay::new(), Vec::new())
}

fn text_source(
    directory: &TempDirectory,
    text: &str,
    configuration: &CsvConfiguration,
) -> LargeCSVSource {
    let path = directory.file("t.csv", text.as_bytes());
    let file = Arc::new(MappedFile::open(&path).expect("file opens"));
    make_source(file, configuration)
}

fn generated_path(
    directory: &TempDirectory,
    target_bytes: usize,
    columns: usize,
    quoted: bool,
    multiline_every: Option<usize>,
    seed: u64,
) -> (PathBuf, usize) {
    let path = directory.path().join("big.csv");
    let (rows, _) = generate_large_csv(
        &path,
        target_bytes,
        columns,
        quoted,
        multiline_every,
        false,
        false,
        seed,
    );
    (path, rows)
}

#[test]
fn cells_equal_in_memory_table() {
    let directory = TempDirectory::new();
    let (path, rows) = generated_path(&directory, 300_000, 5, true, Some(7), 5);
    let file = Arc::new(MappedFile::open(&path).expect("file opens"));
    let bytes = std::fs::read(&path).expect("file reads");
    let mut parser = CsvParser::from_bytes(&bytes, &configuration()).expect("file decodes");
    let table = CsvTable::from_rows(parser.read_all().expect("file parses"), None, 1);

    let source = make_source(file, &configuration());
    assert_eq!(source.row_count(), rows);
    assert_eq!(source.row_count(), table.row_count());
    assert_eq!(source.column_count(), table.columns.len());
    for row in 0..table.row_count() {
        for column in 0..table.columns.len() {
            assert_eq!(
                source.cell(row, column),
                table.cell(row, column),
                "row {row} column {column}"
            );
        }
    }
}

#[test]
fn header_offset() {
    let directory = TempDirectory::new();
    let source = text_source(
        &directory,
        "a,b\n1,2\n3,4\n",
        &CsvConfiguration {
            first_row_as_header: true,
            ..configuration()
        },
    );
    assert_eq!(source.row_count(), 2);
    assert_eq!(
        source.headers(),
        Some(vec!["a".to_string(), "b".to_string()])
    );
    assert_eq!(source.cell(0, 0), "1");
    assert_eq!(source.physical_row(0), 1);
}

#[test]
fn no_header_means_nil_headers() {
    let directory = TempDirectory::new();
    let source = text_source(&directory, "a,b\n", &configuration());
    assert_eq!(source.headers(), None);
}

#[test]
fn ragged_row_pads_with_empty() {
    let directory = TempDirectory::new();
    let source = text_source(&directory, "a,b,c\nx\n", &configuration());
    assert_eq!(source.column_count(), 3);
    assert_eq!(source.cell(1, 0), "x");
    assert_eq!(source.cell(1, 2), "");
    assert_eq!(source.cell(9, 0), "");
    assert_eq!(source.cell(0, 9), "");
}

#[test]
fn column_count_capped_at_2048() {
    let directory = TempDirectory::new();
    let text = format!("{}\n", vec!["x"; 2100].join(","));
    let source = text_source(&directory, &text, &configuration());
    assert_eq!(source.column_count(), LargeFile::MAX_COLUMNS);
    assert_eq!(source.column_ids().len(), LargeFile::MAX_COLUMNS);
}

#[test]
fn column_ids_are_stable() {
    let directory = TempDirectory::new();
    let source = text_source(&directory, "a,b\n", &configuration());
    let ids = source.column_ids();
    assert_eq!(ids, source.column_ids());
    let unique: HashSet<_> = ids.iter().copied().collect();
    assert_eq!(unique.len(), 2);
}

#[test]
fn column_ids_carry_over_to_a_replacing_source() {
    let directory = TempDirectory::new();
    let first = text_source(&directory, "a,b\n", &configuration());
    let path = directory.file("u.csv", b"a,b,c\n");
    let file = Arc::new(MappedFile::open(&path).expect("file opens"));
    let indexer = make_indexer(Arc::clone(&file), &configuration(), STEP_BYTES);
    indexer.start(|_| {});
    indexer.wait();
    let second = source_with(
        file,
        &configuration(),
        indexer,
        CellOverlay::new(),
        first.column_ids(),
    );
    assert_eq!(&second.column_ids()[..2], &first.column_ids()[..]);
    assert_eq!(second.column_ids().len(), 3);
}

#[test]
fn bom_file_row_0() {
    let directory = TempDirectory::new();
    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend_from_slice(b"a,b\n");
    let path = directory.file("b.csv", &bytes);
    let file = Arc::new(MappedFile::open(&path).expect("file opens"));
    let source = make_source(file, &configuration());
    assert!(source.had_utf8_bom);
    assert_eq!(source.cell(0, 0), "a");
}

#[test]
fn lossy_block_rejects_edit() {
    let directory = TempDirectory::new();
    let mut bytes = b"a,b\n".to_vec();
    bytes.extend_from_slice(&[0xFF, b',', b'x', b'\n']);
    let path = directory.file("l.csv", &bytes);
    let file = Arc::new(MappedFile::open(&path).expect("file opens"));
    let source = make_source(file, &configuration());
    assert_eq!(source.cell(1, 0), "\u{FFFD}");
    assert!(!source.is_editable(1));
    assert_eq!(source.set_cell("z", 1, 0), Err(CsvError::RowNotEditable));
}

#[test]
fn lossy_non_utf8_block_uses_latin1_fallback() {
    let directory = TempDirectory::new();
    let mut bytes = b"a,b\n".to_vec();
    bytes.extend_from_slice(&[0x81, b',', b'x', b'\n']);
    let path = directory.file("w.csv", &bytes);
    let file = Arc::new(MappedFile::open(&path).expect("file opens"));
    let configuration = CsvConfiguration {
        encoding: Encoding::Windows1252,
        ..configuration()
    };
    let source = make_source(file, &configuration);
    assert_eq!(source.cell(1, 0), "\u{0081}");
    assert!(!source.is_editable(1));
}

#[test]
fn set_cell_override_and_revert() {
    let directory = TempDirectory::new();
    let source = text_source(&directory, "a,b\n1,2\n", &configuration());
    assert_eq!(source.set_cell("new", 1, 1), Ok(None));
    assert_eq!(source.cell(1, 1), "new");
    assert_eq!(source.overlay.count(), 1);
    assert_eq!(source.set_cell("again", 1, 1), Ok(Some("new".to_string())));
    assert_eq!(source.set_cell("2", 1, 1), Ok(Some("again".to_string())));
    assert!(source.overlay.is_empty());
    assert_eq!(source.cell(1, 1), "2");
}

#[test]
fn edited_cell_key_uses_physical_row() {
    let directory = TempDirectory::new();
    let source = text_source(
        &directory,
        "a,b\n1,2\n",
        &CsvConfiguration {
            first_row_as_header: true,
            ..configuration()
        },
    );
    source.set_cell("new", 0, 0).expect("cell is editable");
    assert_eq!(
        source.overlay.get(CellKey { row: 1, column: 0 }),
        Some("new".to_string())
    );
}

#[test]
fn overlay_wins_over_the_file_bytes() {
    let directory = TempDirectory::new();
    let source = text_source(&directory, "a,b\n1,2\n", &configuration());
    source
        .overlay
        .set("edited".to_string(), CellKey { row: 1, column: 0 });
    assert_eq!(source.cell(1, 0), "edited");
    assert_eq!(source.cell(1, 1), "2");
}

#[test]
fn set_cell_rejects_ambiguous_when_quotes_off() {
    let directory = TempDirectory::new();
    let configuration = CsvConfiguration {
        column_separator: ";".to_string(),
        quote_character: String::new(),
        ..configuration()
    };
    let source = text_source(&directory, "a;b\n1;2\n", &configuration);
    assert_eq!(source.set_cell("x;y", 0, 0), Err(CsvError::AmbiguousOutput));
    assert!(source.overlay.is_empty());
}

#[test]
fn set_cell_rejects_unencodable_text() {
    let directory = TempDirectory::new();
    let configuration = CsvConfiguration {
        encoding: Encoding::Windows1252,
        ..configuration()
    };
    let source = text_source(&directory, "a,b\n1,2\n", &configuration);
    assert_eq!(
        source.set_cell("\u{1F600}", 0, 0),
        Err(CsvError::UnencodableText)
    );
    assert!(source.overlay.is_empty());
}

#[test]
fn set_cell_rejects_out_of_range() {
    let directory = TempDirectory::new();
    let source = text_source(&directory, "a,b\n", &configuration());
    assert_eq!(source.set_cell("x", 5, 0), Err(CsvError::RowNotEditable));
    assert_eq!(source.set_cell("x", 0, 5), Err(CsvError::RowNotEditable));
}

#[test]
fn set_cell_rejected_while_indexing() {
    let directory = TempDirectory::new();
    let (path, _) = generated_path(&directory, 300_000, 3, false, None, 1);
    let file = Arc::new(MappedFile::open(&path).expect("file opens"));
    let source = partial_source(file, &configuration(), STEP_BYTES);
    assert!(source.row_count() > 0);
    assert!(!source.is_editable(0));
    assert_eq!(source.set_cell("x", 0, 0), Err(CsvError::RowNotEditable));
}

#[test]
fn partial_index_hides_the_row_still_being_scanned() {
    let directory = TempDirectory::new();
    let (path, _) = generated_path(&directory, 300_000, 3, false, None, 1);
    let file = Arc::new(MappedFile::open(&path).expect("file opens"));
    let source = partial_source(file, &configuration(), 4000);
    let last = source.row_count() - 1;
    assert!(source.cell(last, 2).ends_with(".2"));
    assert_eq!(source.cell(last + 1, 0), "");
}

#[cfg(unix)]
#[test]
fn stale_file_returns_empty_cells() {
    let directory = TempDirectory::new();
    let (path, _) = generated_path(&directory, 300_000, 3, false, None, 1);
    let file = Arc::new(MappedFile::open(&path).expect("file opens"));
    let source = make_source(file, &configuration());
    assert!(!source.cell(2000, 0).is_empty());

    let handle = std::fs::OpenOptions::new()
        .write(true)
        .open(&path)
        .expect("file opens for truncation");
    handle.set_len(100).expect("file is truncated");

    assert_eq!(source.staleness(), Staleness::ReplacedOrTruncated);
    assert_eq!(source.cell(3000, 0), "");
    assert!(!source.is_editable(0));
}

#[cfg(unix)]
#[test]
fn replaced_file_stays_readable_through_the_mapping() {
    let directory = TempDirectory::new();
    let (path, _) = generated_path(&directory, 300_000, 3, false, None, 1);
    let bytes = std::fs::read(&path).expect("file reads");
    let mut parser = CsvParser::from_bytes(&bytes, &configuration()).expect("file decodes");
    let rows = parser.read_all().expect("file parses");

    let file = Arc::new(MappedFile::open(&path).expect("file opens"));
    let source = make_source(file, &configuration());
    let replacement = directory.file("new.csv", b"x\n");
    replace_file(&path, &replacement).expect("replacement succeeds");

    assert_eq!(source.staleness(), Staleness::ReplacedOrTruncated);
    assert_eq!(source.cell(3000, 0), rows[3000][0]);
    assert!(!source.is_editable(0));
}

#[test]
fn block_cache_hit_rate_on_sequential_read() {
    let directory = TempDirectory::new();
    let (path, _) = generated_path(&directory, 300_000, 4, false, None, 1);
    let file = Arc::new(MappedFile::open(&path).expect("file opens"));
    let source = make_source(file, &configuration());
    for row in 0..source.row_count() {
        for column in 0..source.column_count() {
            let _ = source.cell(row, column);
        }
    }
    let (hits, misses) = source.cache_stats();
    let total = hits + misses;
    assert!(total > 0);
    assert!(
        hits as f64 / total as f64 > 0.95,
        "hit rate was {hits}/{total}"
    );
    assert!(misses <= source.row_count() / LargeFile::BLOCK_ROWS + 2);
}

/// The scanner does not break rows at U+2028, but the parser that decodes a
/// block does. A block holding one decodes into more rows than the index
/// recorded; it must become unavailable instead of shifting every later row.
#[test]
fn unicode_line_separator_block_is_unavailable_and_read_only() {
    let directory = TempDirectory::new();
    let rows: Vec<String> = (0..70).map(|row| format!("r{row},v{row}")).collect();
    let text = rows.join("\n").replace("r1,v1", "r1a\u{2028}r1b,v1");
    let source = text_source(&directory, &text, &configuration());

    assert_eq!(source.row_count(), 70, "the scanner counts only LF rows");
    assert_eq!(source.cell(0, 0), "", "the unaligned block maps no rows");
    assert_eq!(source.cell(0, 1), "");
    assert!(!source.is_editable(0));
    assert_eq!(source.set_cell("x", 0, 0), Err(CsvError::RowNotEditable));

    assert_eq!(
        source.cell(64, 0),
        "r64",
        "the following block still maps to its physical rows"
    );
    assert!(source.is_editable(64));
    source
        .set_cell("edited", 64, 1)
        .expect("an aligned block after the separator stays editable");
}

#[test]
fn malformed_block_shows_raw_line() {
    let directory = TempDirectory::new();
    let source = text_source(&directory, "a,b\nx\"y\"z,2\nc,d\n", &configuration());
    assert_eq!(source.cell(1, 0), "x\"y\"z,2");
    assert_eq!(source.cell(1, 1), "");
    assert_eq!(source.cell(2, 0), "c,d");
    assert!(!source.is_editable(1));
}

#[test]
fn header_can_be_toggled_afterwards() {
    let directory = TempDirectory::new();
    let source = text_source(&directory, "a,b\n1,2\n3,4\n", &configuration());
    assert_eq!(source.row_count(), 3);
    assert_eq!(source.headers(), None);
    source.set_header_enabled(true);
    assert_eq!(source.row_count(), 2);
    assert_eq!(
        source.headers(),
        Some(vec!["a".to_string(), "b".to_string()])
    );
    assert_eq!(source.cell(0, 0), "1");
    source.set_header_enabled(false);
    assert_eq!(source.row_count(), 3);
    assert_eq!(source.cell(0, 0), "a");
}

#[test]
fn cells_straddle_block_boundaries() {
    let directory = TempDirectory::new();
    let (path, rows) = generated_path(&directory, 10_000, 3, false, None, 9);
    assert!(rows > 130);
    let bytes = std::fs::read(&path).expect("file reads");
    let mut parser = CsvParser::from_bytes(&bytes, &configuration()).expect("file decodes");
    let table = CsvTable::from_rows(parser.read_all().expect("file parses"), None, 1);
    let file = Arc::new(MappedFile::open(&path).expect("file opens"));
    let source = make_source(file, &configuration());

    for row in [61, 62, 63, 64, 65, 127, 128, source.row_count() - 1] {
        for column in 0..table.columns.len() {
            assert_eq!(
                source.cell(row, column),
                table.cell(row, column),
                "row {row} column {column}"
            );
        }
    }
}

#[test]
fn empty_file_is_one_empty_row() {
    let directory = TempDirectory::new();
    let source = text_source(&directory, "", &configuration());
    assert_eq!(source.row_count(), 1);
    assert_eq!(source.column_count(), 1);
    assert_eq!(source.cell(0, 0), "");
    assert_eq!(source.indexing_fraction(), 1.0);
    assert!(source.is_indexed());
}

#[test]
fn indexing_fraction_tracks_the_scan() {
    let directory = TempDirectory::new();
    let empty = text_source(&directory, "", &configuration());
    assert_eq!(empty.indexing_fraction(), 1.0);

    let (path, _) = generated_path(&directory, 300_000, 3, false, None, 1);
    let full_file = Arc::new(MappedFile::open(&path).expect("file opens"));
    let full = make_source(full_file, &configuration());
    assert_eq!(full.indexing_fraction(), 1.0);

    let partial_file = Arc::new(MappedFile::open(&path).expect("file opens"));
    let partial = partial_source(partial_file, &configuration(), STEP_BYTES);
    let fraction = partial.indexing_fraction();
    assert!(fraction > 0.0 && fraction < 1.0, "fraction was {fraction}");
    assert!(!partial.is_indexed());
}

#[test]
fn save_snapshot_captures_edits_and_index() {
    let directory = TempDirectory::new();
    let source = text_source(&directory, "a,b\n1,2\n", &configuration());
    source.set_cell("new", 1, 1).expect("cell is editable");
    let snapshot = source.save_snapshot();
    assert_eq!(snapshot.file.size(), 8);
    assert_eq!(snapshot.index.scanned_bytes, 8);
    assert_eq!(snapshot.configuration, configuration());
    assert!(!snapshot.had_bom);
    assert_eq!(
        snapshot.edits,
        HashMap::from([(CellKey { row: 1, column: 1 }, "new".to_string())])
    );

    source.overlay.remove_all();
    assert_eq!(snapshot.edits.len(), 1);
}

#[test]
fn headers_redecode_when_the_scan_advances() {
    let directory = TempDirectory::new();
    let (path, _) = generated_path(&directory, 2_000, 3, false, None, 9);
    let file = Arc::new(MappedFile::open(&path).expect("file opens"));
    let configuration = CsvConfiguration {
        first_row_as_header: true,
        ..configuration()
    };
    let indexer = make_indexer(Arc::clone(&file), &configuration, 1);
    let slot: Arc<Mutex<Option<LargeCSVSource>>> = Arc::new(Mutex::new(None));
    let sink = Arc::clone(&slot);
    let scan = Arc::clone(&indexer);
    indexer.start(move |_| {
        let mut slot = sink.lock().expect("source slot lock");
        if slot.is_none() {
            *slot = Some(LargeCSVSource::new(
                Arc::clone(&file),
                configuration.clone(),
                Arc::clone(&scan),
                CellOverlay::new(),
                Vec::new(),
            ));
            assert_eq!(
                slot.as_ref().expect("source was stored").headers(),
                Some(vec!["c".to_string()]),
                "the first one-byte step decodes only the start of the header"
            );
        }
    });
    indexer.wait();

    let source = slot
        .lock()
        .expect("source slot lock")
        .take()
        .expect("source was stored");
    assert_eq!(
        source.headers(),
        Some(vec![
            "col0".to_string(),
            "col1".to_string(),
            "col2".to_string()
        ]),
        "the cached partial block must be decoded again after the scan advances"
    );
}

#[test]
fn empty_file_with_header_hides_the_only_row() {
    let directory = TempDirectory::new();
    let path = directory.file("e.csv", b"");
    let file = Arc::new(MappedFile::open(&path).expect("file opens"));
    let configuration = CsvConfiguration {
        first_row_as_header: true,
        ..configuration()
    };
    let indexer = make_indexer(Arc::clone(&file), &configuration, STEP_BYTES);
    let source = LargeCSVSource::new(
        Arc::clone(&file),
        configuration,
        Arc::clone(&indexer),
        CellOverlay::new(),
        Vec::new(),
    );
    indexer.start(|_| {});
    indexer.wait();

    assert_eq!(source.row_count(), 0);
    assert_eq!(source.headers(), Some(vec![String::new()]));
    assert_eq!(source.cell(0, 0), "");
}

#[test]
fn large_source_is_a_row_source() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<LargeCSVSource>();

    let directory = TempDirectory::new();
    let source = text_source(&directory, "a,b\n1,2\n", &configuration());
    let view: &dyn RowSource = &source;
    assert_eq!(view.row_count(), 2);
    assert_eq!(view.column_count(), 2);
    assert_eq!(view.column_ids(), source.column_ids());
    assert_eq!(view.headers(), None);
    assert_eq!(view.cell(1, 1), "2");
}
