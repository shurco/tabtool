//! Port of `CSVDocumentLargeTests.swift` and the large-mode routing cases for
//! TabTool's Rust document.
//!
//! A large document owns a mapped file and a growing row index; it never owns
//! an in-memory table. `open_large` maps, checks or detects the format and
//! starts indexing. `open_file` routes by the file size against the threshold:
//! at or above it the file opens large, below it stays in memory.

use std::io::Write;
use std::path::{Path, PathBuf};

use tabtool_core::{CsvConfiguration, CsvError};
use tabtool_document::{
    Document, DocumentError, DocumentPhase, GridSelection, LargeStatus, StdStorage,
};
use tabtool_large::LargeFile;

/// A temporary directory that outlives the mapped file.
struct Fixture {
    directory: tempfile::TempDir,
}

impl Fixture {
    fn new() -> Self {
        Self {
            directory: tempfile::tempdir().expect("temporary directory"),
        }
    }

    fn path(&self) -> PathBuf {
        self.directory.path().join("big.csv")
    }

    fn write(&self, bytes: &[u8]) -> PathBuf {
        let path = self.path();
        std::fs::write(&path, bytes).expect("fixture file is written");
        path
    }

    /// Rows like `r5,v5,"q 5"`; the second field repeats every seven rows.
    fn csv(&self, rows: usize) -> PathBuf {
        let text = (0..rows)
            .map(|row| format!("r{row},v{},\"q {row}\"", row % 7))
            .collect::<Vec<_>>()
            .join("\n");
        self.write(text.as_bytes())
    }

    fn append_row(&self, row: usize) {
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(self.path())
            .expect("append the fixture");
        write!(file, "\nr{row},v{},\"q {row}\"", row % 7).expect("append a row");
    }
}

/// Opens `path` large and waits for the index, leaving the status `Ready`.
fn open_ready(path: &Path, configuration: Option<CsvConfiguration>) -> Document {
    let mut document = Document::new(configuration.clone().unwrap_or_default());
    document
        .open_large(path, configuration, false, 0)
        .expect("open the large fixture");
    finish_indexing(&mut document);
    document
}

/// Blocks until the current indexer has finished, then publishes its status.
fn finish_indexing(document: &mut Document) {
    if let Some(state) = document.large_state() {
        state.indexer.wait();
    }
    document.poll_large_indexing();
}

/// Opens `path` through the size-routing entry point.
fn open_routed(path: &Path, threshold_bytes: u64) -> Document {
    let mut document = Document::new(CsvConfiguration::default());
    document
        .open_file(path, &StdStorage, None, false, threshold_bytes)
        .expect("open the fixture through the router");
    if document.is_large_mode() {
        finish_indexing(&mut document);
    }
    document
}

/// A ready large document with no header row, so data indexes match the file.
fn open_unheaded(path: &Path) -> Document {
    let mut document = open_ready(path, None);
    if document.configuration().first_row_as_header {
        document.set_large_header_enabled(false);
    }
    document
}

/// The cell text the grid reads for `row`/`column`.
fn cell(document: &Document, row: usize, column: usize) -> String {
    document
        .row_source()
        .expect("the document has a row source")
        .cell(row, column)
}

#[test]
fn uses_large_mode_at_and_above_the_threshold() {
    assert!(Document::uses_large_mode(1_000, 1_000));
    assert!(!Document::uses_large_mode(999, 1_000));
    assert!(Document::uses_large_mode(
        u64::MAX,
        LargeFile::THRESHOLD_BYTES
    ));
}

#[test]
fn documented_threshold_is_one_hundred_mebibytes() {
    assert_eq!(LargeFile::THRESHOLD_BYTES, 100 * 1_048_576);
    assert!(Document::uses_large_mode(
        LargeFile::THRESHOLD_BYTES,
        LargeFile::THRESHOLD_BYTES
    ));
    assert!(!Document::uses_large_mode(
        LargeFile::THRESHOLD_BYTES - 1,
        LargeFile::THRESHOLD_BYTES
    ));
}

#[test]
fn routes_a_file_at_or_above_the_threshold_to_large_mode() {
    let fixture = Fixture::new();
    let path = fixture.csv(50);
    let size = std::fs::metadata(&path).expect("fixture metadata").len();

    let document = open_routed(&path, size);

    assert!(document.is_large_mode());
    assert!(document.table().is_none());
    assert_eq!(document.large_status(), LargeStatus::Ready);
    assert_eq!(document.phase(), &DocumentPhase::Ready);
    assert_eq!(document.file_path(), Some(path.as_path()));
}

#[test]
fn routes_a_file_below_the_threshold_to_memory() {
    let fixture = Fixture::new();
    let path = fixture.csv(50);
    let size = std::fs::metadata(&path).expect("fixture metadata").len();

    let document = open_routed(&path, size + 1);

    assert!(!document.is_large_mode());
    assert!(document.large_state().is_none());
    assert_eq!(document.large_status(), LargeStatus::None);
    assert!(document.table().is_some());
    assert_eq!(document.phase(), &DocumentPhase::Ready);
}

#[test]
fn opens_large_and_publishes_the_source() {
    let fixture = Fixture::new();
    let document = open_ready(&fixture.csv(5_000), None);

    assert!(document.table().is_none());
    assert!(document.is_large_mode());
    assert!(document.has_content());
    assert_eq!(document.phase(), &DocumentPhase::Ready);
    assert_eq!(document.large_status(), LargeStatus::Ready);

    let header_rows = usize::from(document.configuration().first_row_as_header);
    let source = document.row_source().expect("large row source");
    assert_eq!(source.row_count(), 5_000 - header_rows);
    assert_eq!(source.column_count(), 3);
    assert_eq!(source.cell(source.row_count() - 1, 0), "r4999".to_string());
}

#[test]
fn a_large_document_has_no_table_and_a_memory_document_has_no_large_state() {
    let fixture = Fixture::new();
    let path = fixture.csv(20);

    let large = open_ready(&path, None);
    assert!(large.is_large_mode());
    assert!(large.table().is_none());

    let memory = open_routed(&path, u64::MAX);
    assert!(!memory.is_large_mode());
    assert!(memory.large_state().is_none());
    assert!(memory.table().is_some());
}

#[test]
fn new_document_clears_the_large_mode() {
    let fixture = Fixture::new();
    let mut document = open_ready(&fixture.csv(20), None);
    assert!(document.is_large_mode());

    document
        .new_document(CsvConfiguration::default())
        .expect("replace with a new document");

    assert!(!document.is_large_mode());
    assert!(document.large_state().is_none());
    assert_eq!(document.large_status(), LargeStatus::None);
    assert!(document.table().is_some());
}

#[test]
fn small_documents_stay_in_memory() {
    let mut document = Document::new(CsvConfiguration::default());

    document
        .load_from_bytes(b"a,b\n1,2\n", None, None, false)
        .expect("load the fixture");

    assert!(!document.is_large_mode());
    assert!(document.table().is_some());
    assert!(document.large_state().is_none());
    assert_eq!(document.large_status(), LargeStatus::None);
}

#[test]
fn utf16_files_report_a_read_failure() {
    let fixture = Fixture::new();
    let mut bytes = vec![0xFF, 0xFE];
    bytes.extend("a,b\n1,2\n".encode_utf16().flat_map(u16::to_le_bytes));
    let path = fixture.write(&bytes);
    let mut document = Document::new(CsvConfiguration::default());

    assert_eq!(
        document.open_large(&path, None, false, 0),
        Err(DocumentError::Csv(CsvError::UnsupportedLargeFileEncoding))
    );

    assert_eq!(
        document.phase(),
        &DocumentPhase::ReadFailed(CsvError::UnsupportedLargeFileEncoding)
    );
    assert!(document.large_state().is_none());
    assert!(document.table().is_none());
    assert_eq!(document.large_status(), LargeStatus::None);
    assert_eq!(document.file_path(), Some(path.as_path()));
}

#[test]
fn a_missing_large_file_is_unreadable() {
    let fixture = Fixture::new();
    let path = fixture.path();
    let mut document = Document::new(CsvConfiguration::default());

    assert_eq!(
        document.open_large(&path, None, false, 0),
        Err(DocumentError::CannotReadFile)
    );

    assert_eq!(document.phase(), &DocumentPhase::UnreadableFile);
    assert!(document.large_state().is_none());
    assert!(document.table().is_none());
    assert_eq!(document.large_status(), LargeStatus::None);
    assert_eq!(document.file_path(), Some(path.as_path()));
}

#[test]
fn open_large_refuses_a_file_below_the_threshold_without_touching_the_document() {
    let fixture = Fixture::new();
    let path = fixture.csv(20);
    let size = std::fs::metadata(&path).expect("fixture metadata").len();
    let mut document = Document::new(CsvConfiguration::default());
    let revision = document.revision();

    assert_eq!(
        document.open_large(&path, None, false, size + 1),
        Err(DocumentError::NotReady)
    );

    assert!(!document.is_large_mode());
    assert!(document.table().is_some());
    assert_eq!(document.revision(), revision);
    assert!(document.file_path().is_none());
}

#[test]
fn the_row_source_is_available_before_the_index_finishes() {
    let fixture = Fixture::new();
    let path = fixture.csv(20_000);
    let mut document = Document::new(CsvConfiguration::default());
    let configuration = CsvConfiguration::default();

    document
        .open_large(&path, Some(configuration), false, 0)
        .expect("open the large fixture");

    // The source is published as soon as the mapping exists; the rows whose
    // end is known are readable while the rest of the file is still scanned.
    let source = document.row_source().expect("immediate row source");
    assert!(source.row_count() <= 20_000);

    finish_indexing(&mut document);
    assert_eq!(document.large_status(), LargeStatus::Ready);
    assert_eq!(
        document.row_source().expect("finished source").row_count(),
        20_000
    );
    assert_eq!(
        document
            .row_source()
            .expect("finished source")
            .cell(19_999, 0),
        "r19999".to_string()
    );
}

#[test]
fn structural_commands_are_inert_in_large_mode() {
    let fixture = Fixture::new();
    let mut document = open_unheaded(&fixture.csv(30));
    let row_count = document.row_source().expect("source").row_count();
    let column_count = document.row_source().expect("source").column_count();
    let revision = document.revision();
    let locale: tabtool_core::Locale = "en-US".parse().expect("locale");
    *document.selection_mut() = GridSelection {
        rows: std::collections::BTreeSet::from([1]),
        columns: std::collections::BTreeSet::from([1]),
    };

    assert!(!document.can_delete_selection());
    assert!(!document.can_rename_column(0));
    assert!(!document.can_move_selected_rows(1));
    assert!(!document.can_insert_rows());

    document.add_row_above_selection();
    document.add_row_below_selection();
    document.add_column_left_of_selection();
    document.add_column_right_of_selection();
    document.delete_selection();
    document.delete_rows(&std::collections::BTreeSet::from([0]));
    document.delete_columns(&std::collections::BTreeSet::from([0]));
    document.move_rows(&std::collections::BTreeSet::from([0]), 2);
    document.move_columns(&std::collections::BTreeSet::from([0]), 2);
    document.move_selected_rows(1);
    document.move_selected_columns(1);
    document.rename_column(0, "changed");
    document.sort_rows(0, tabtool_core::SortDirection::Ascending, &locale);
    document.paste_tsv("x\ty", &locale, ".");
    document.set_decimal_mark(",");

    assert_eq!(document.revision(), revision);
    assert!(document.table().is_none());
    assert_eq!(
        document.row_source().expect("source").row_count(),
        row_count
    );
    assert_eq!(
        document.row_source().expect("source").column_count(),
        column_count
    );
    assert_eq!(cell(&document, 0, 0), "r0");
    assert!(document.snapshot().is_err());
    assert!(document.encoded_data().is_err());
    assert!(document.visible_table().is_none());

    // Cells stay editable: the generic cell command routes to the overlay.
    assert!(document.can_edit_cells());
    document.set_cell(0, 0, "changed");
    assert_eq!(cell(&document, 0, 0), "changed");
    assert_eq!(document.revision(), revision + 1);
}

#[test]
fn an_appended_file_reports_stale_and_blocks_edits() {
    let fixture = Fixture::new();
    let path = fixture.csv(30);
    let mut document = open_ready(&path, None);
    assert!(document.can_edit_cells());

    fixture.append_row(30);
    document.refresh_large_staleness();

    assert_eq!(
        document.large_status(),
        LargeStatus::Stale { appended: true }
    );
    assert_eq!(document.phase(), &DocumentPhase::Ready);
    assert!(!document.can_edit_cells());
    assert_eq!(
        document.set_large_cell("X", 0, 0),
        Err(CsvError::RowNotEditable)
    );
    assert_eq!(
        document.notice(),
        Some(tabtool_document::DocumentNotice::EditRejected(
            CsvError::RowNotEditable
        ))
    );
}

#[test]
#[cfg(unix)] // Truncating a mapped file in place is Unix-only, as in tabtool-large.
fn a_replaced_or_truncated_file_reports_stale() {
    let fixture = Fixture::new();
    let path = fixture.csv(30);
    let mut document = open_ready(&path, None);

    fixture.write(b"short");
    document.refresh_large_staleness();

    assert_eq!(
        document.large_status(),
        LargeStatus::Stale { appended: false }
    );
    assert_eq!(document.phase(), &DocumentPhase::Ready);
    assert!(!document.can_edit_cells());
}

#[test]
fn polling_keeps_a_stale_status_and_a_ready_phase() {
    let fixture = Fixture::new();
    let path = fixture.csv(30);
    let mut document = open_ready(&path, None);
    fixture.append_row(30);
    document.refresh_large_staleness();

    document.poll_large_indexing();

    assert_eq!(
        document.large_status(),
        LargeStatus::Stale { appended: true }
    );
    assert_eq!(document.phase(), &DocumentPhase::Ready);
}

#[test]
fn reload_reopens_the_file_and_keeps_the_overlay() {
    let fixture = Fixture::new();
    let path = fixture.csv(30);
    let mut document = open_unheaded(&path);
    document.set_large_cell("X", 2, 1).expect("edit the cell");
    *document.selection_mut() = GridSelection {
        rows: std::collections::BTreeSet::from([2]),
        columns: std::collections::BTreeSet::new(),
    };
    fixture.append_row(30);
    document.refresh_large_staleness();
    assert!(matches!(
        document.large_status(),
        LargeStatus::Stale { appended: true }
    ));

    document.reload_large(&StdStorage).expect("reload");

    finish_indexing(&mut document);
    assert_eq!(document.large_status(), LargeStatus::Ready);
    assert_eq!(document.row_source().expect("source").row_count(), 31);
    assert_eq!(cell(&document, 2, 1), "X");
    assert!(document.has_unsaved_changes());
    assert_eq!(
        document.selection().rows,
        std::collections::BTreeSet::from([2])
    );
    assert_eq!(
        document
            .large_state()
            .expect("large")
            .source
            .overlay
            .count(),
        1
    );

    // The undo history survives a reload; the targeted entry still addresses
    // the same file row.
    assert!(document.undo());
    assert_eq!(cell(&document, 2, 1), "v2");
    assert!(!document.has_unsaved_changes());
}

#[test]
fn reload_without_a_path_or_a_source_is_not_ready() {
    let mut document = Document::new(CsvConfiguration::default());
    assert_eq!(
        document.reload_large(&StdStorage),
        Err(DocumentError::NotReady)
    );
}

#[test]
fn detect_format_reads_the_head_of_a_large_file() {
    let fixture = Fixture::new();
    let document = open_ready(&fixture.csv(100), None);

    let detected = document
        .detect_format_from_disk(&StdStorage, false)
        .expect("detect the format");

    assert_eq!(detected.column_separator, ",");
    assert_eq!(document.configuration().column_separator, ",");
}

#[test]
#[cfg(unix)] // Removing a mapped file is Unix-only, as in tabtool-large.
fn a_failed_reload_keeps_the_current_source() {
    let fixture = Fixture::new();
    let path = fixture.csv(30);
    let mut document = open_ready(&path, None);
    let before = std::sync::Arc::clone(&document.large_state().expect("large").source);
    std::fs::remove_file(&path).expect("remove the fixture");

    assert_eq!(
        document.reload_large(&StdStorage),
        Err(DocumentError::CannotReadFile)
    );

    assert!(std::sync::Arc::ptr_eq(
        &before,
        &document.large_state().expect("large").source
    ));
    assert_eq!(document.large_status(), LargeStatus::Ready);
}
