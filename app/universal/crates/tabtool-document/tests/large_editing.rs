//! Port of `CSVDocumentLargeEditingTests.swift` for TabTool's Rust document.
//!
//! Large-file edits go through `LargeCSVSource::set_cell` and live in the
//! overlay. The document records a targeted cell change for undo, counts the
//! overlay as the unsaved state, and refuses edits while the index is
//! incomplete or the file changed on disk. The header toggle is a view mapping
//! with one undo step; revert drops the overlay and the undo history.

use std::collections::BTreeSet;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use tabtool_core::{CsvConfiguration, CsvError, Encoding, Locale};
use tabtool_document::{
    Document, DocumentError, DocumentNotice, DocumentPhase, GridSelection, LargeStatus,
    RevertDecision, StdStorage, UndoAction, plan_revert,
};

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

fn open_ready(path: &Path, configuration: Option<CsvConfiguration>) -> Document {
    let mut document = Document::new(configuration.clone().unwrap_or_default());
    document
        .open_large(path, configuration, false, 0)
        .expect("open the large fixture");
    finish_indexing(&mut document);
    document
}

fn finish_indexing(document: &mut Document) {
    if let Some(state) = document.large_state() {
        state.indexer.wait();
    }
    document.poll_large_indexing();
}

/// A ready large document with no header row, like the Swift `open` helper.
fn open_without_header(path: &Path) -> Document {
    open_with_configuration(path, None)
}

fn open_with_configuration(path: &Path, configuration: Option<CsvConfiguration>) -> Document {
    let mut document = open_ready(path, configuration);
    if document.configuration().first_row_as_header {
        document.set_large_header_enabled(false);
    }
    document
}

fn cell(document: &Document, row: usize, column: usize) -> String {
    document
        .row_source()
        .expect("the document has a row source")
        .cell(row, column)
}

fn source(document: &Document) -> Arc<tabtool_large::LargeCSVSource> {
    Arc::clone(&document.large_state().expect("large state").source)
}

fn rows(indexes: impl IntoIterator<Item = usize>) -> BTreeSet<usize> {
    indexes.into_iter().collect()
}

fn locale() -> Locale {
    "en-US".parse().expect("test locale is valid")
}

#[test]
fn edit_registers_undo_and_redo() {
    let fixture = Fixture::new();
    let mut document = open_without_header(&fixture.csv(300));
    let large = source(&document);

    document.set_large_cell("X", 2, 1).expect("edit the cell");

    assert_eq!(large.cell(2, 1), "X");
    assert_eq!(large.overlay.count(), 1);
    assert_eq!(document.undo_action(), Some(UndoAction::EditCell));
    assert!(document.has_unsaved_changes());

    assert!(document.undo());
    assert_eq!(large.cell(2, 1), "v2");
    assert_eq!(large.overlay.count(), 0);
    assert!(!document.has_unsaved_changes());

    assert!(document.redo());
    assert_eq!(large.cell(2, 1), "X");
    assert_eq!(large.overlay.count(), 1);
    assert!(document.has_unsaved_changes());
}

#[test]
fn the_generic_set_cell_edits_a_large_document() {
    let fixture = Fixture::new();
    let mut document = open_without_header(&fixture.csv(30));

    document.set_cell(2, 1, "X");

    assert_eq!(cell(&document, 2, 1), "X");
    assert_eq!(document.undo_action(), Some(UndoAction::EditCell));
    assert!(document.undo());
    assert_eq!(cell(&document, 2, 1), "v2");
}

#[test]
fn edit_back_to_original_is_clean() {
    let fixture = Fixture::new();
    let mut document = open_without_header(&fixture.csv(30));

    document.set_large_cell("X", 0, 0).expect("edit the cell");
    assert!(document.has_unsaved_changes());

    document
        .set_large_cell("r0", 0, 0)
        .expect("restore the cell");

    assert!(!document.has_unsaved_changes());
    assert!(!document.is_dirty());
    assert!(
        document
            .large_state()
            .expect("large")
            .source
            .overlay
            .is_empty()
    );
}

#[test]
#[cfg(unix)] // Truncating a mapped file in place is Unix-only, as in tabtool-large.
fn edit_rejected_when_the_file_changed_and_registers_no_undo() {
    let fixture = Fixture::new();
    let path = fixture.csv(300);
    let mut document = open_ready(&path, None);
    fixture.write(b"short");

    assert_eq!(
        document.set_large_cell("X", 0, 0),
        Err(CsvError::RowNotEditable)
    );
    assert_eq!(
        document.notice(),
        Some(DocumentNotice::EditRejected(CsvError::RowNotEditable))
    );
    assert!(!document.can_undo());
    assert!(
        document
            .large_state()
            .expect("large")
            .source
            .overlay
            .is_empty()
    );
}

#[test]
fn revert_drops_the_overlay_and_undo() {
    let fixture = Fixture::new();
    let mut document = open_without_header(&fixture.csv(30));
    document.set_large_cell("X", 1, 1).expect("edit the cell");
    assert!(document.has_unsaved_changes());

    document.revert_large_edits();

    assert!(
        document
            .large_state()
            .expect("large")
            .source
            .overlay
            .is_empty()
    );
    assert_eq!(cell(&document, 1, 1), "v1");
    assert!(!document.can_undo());
    assert!(!document.has_unsaved_changes());
}

#[test]
fn header_toggle_changes_row_mapping_and_is_undoable() {
    let fixture = Fixture::new();
    let mut document = open_without_header(&fixture.csv(50));
    assert_eq!(document.row_source().expect("source").row_count(), 50);

    document.set_large_header_enabled(true);

    assert!(document.configuration().first_row_as_header);
    assert_eq!(document.row_source().expect("source").row_count(), 49);
    assert_eq!(
        document.row_source().expect("source").headers(),
        Some(vec!["r0".to_string(), "v0".to_string(), "q 0".to_string()])
    );
    assert_eq!(document.undo_action(), Some(UndoAction::ToggleHeader));
    // The header mapping is view state: it is undoable but not an unsaved edit.
    assert!(!document.has_unsaved_changes());

    assert!(document.undo());
    assert!(!document.configuration().first_row_as_header);
    assert_eq!(document.row_source().expect("source").row_count(), 50);
}

#[test]
fn undo_survives_a_header_toggle() {
    let fixture = Fixture::new();
    let mut document = open_without_header(&fixture.csv(50));
    document.set_large_cell("X", 3, 0).expect("edit the cell");
    document.set_large_header_enabled(true);

    assert!(document.undo());
    assert!(!document.configuration().first_row_as_header);
    assert!(document.undo());

    assert!(
        document
            .large_state()
            .expect("large")
            .source
            .overlay
            .is_empty()
    );
    assert_eq!(cell(&document, 3, 0), "r3");
}

#[test]
fn copies_selected_rows_as_tsv() {
    let fixture = Fixture::new();
    let document = open_without_header(&fixture.csv(30));

    assert_eq!(
        document.copy_large_selection(&rows([1, 2]), &BTreeSet::new(), "."),
        Ok("r1\tv1\tq 1\nr2\tv2\tq 2".to_string())
    );
}

#[test]
fn selected_rows_win_over_selected_columns() {
    let fixture = Fixture::new();
    let document = open_without_header(&fixture.csv(30));

    assert_eq!(
        document.copy_large_selection(&rows([1]), &rows([0]), "."),
        Ok("r1\tv1\tq 1".to_string())
    );
}

#[test]
fn the_generic_copy_also_reads_the_large_source() {
    let fixture = Fixture::new();
    let mut document = open_without_header(&fixture.csv(30));
    *document.selection_mut() = GridSelection {
        rows: rows([1, 2]),
        columns: BTreeSet::new(),
    };

    assert_eq!(
        document.copy_selection(&locale(), "."),
        Some("r1\tv1\tq 1\nr2\tv2\tq 2".to_string())
    );
}

#[test]
fn copies_selected_columns_across_every_row() {
    let fixture = Fixture::new();
    let document = open_without_header(&fixture.csv(3));

    assert_eq!(
        document.copy_large_selection(&BTreeSet::new(), &rows([1]), "."),
        Ok("v0\nv1\nv2".to_string())
    );
}

#[test]
fn copy_over_the_cell_limit_is_refused() {
    let fixture = Fixture::new();
    let document = open_without_header(&fixture.csv(340_000));

    let result = document.copy_large_selection(&rows(0..340_000), &BTreeSet::new(), ".");

    assert_eq!(result, Err(CsvError::RowNotEditable));
    assert_eq!(document.notice(), Some(DocumentNotice::CopyLimitExceeded));
}

#[test]
fn copy_without_a_selection_is_refused() {
    let fixture = Fixture::new();
    let document = open_without_header(&fixture.csv(3));

    assert_eq!(
        document.copy_large_selection(&BTreeSet::new(), &BTreeSet::new(), "."),
        Err(CsvError::RowNotEditable)
    );
    assert_eq!(document.notice(), None);
}

#[test]
fn columns_copy_respects_a_visible_filter() {
    let fixture = Fixture::new();
    let mut document = open_without_header(&fixture.csv(3));
    document.row_filter_mut().visible_rows = Some(vec![1]);

    assert_eq!(
        document.copy_large_selection(&BTreeSet::new(), &rows([1]), "."),
        Ok("v1".to_string())
    );
}

#[test]
fn ambiguous_text_is_rejected_and_records_nothing() {
    let fixture = Fixture::new();
    let configuration = CsvConfiguration {
        quote_character: String::new(),
        escape_character: String::new(),
        ..CsvConfiguration::default()
    };
    let mut document = open_with_configuration(&fixture.csv(30), Some(configuration));
    let revision = document.revision();

    assert_eq!(
        document.set_large_cell("a,b", 0, 0),
        Err(CsvError::AmbiguousOutput)
    );

    assert_eq!(
        document.notice(),
        Some(DocumentNotice::EditRejected(CsvError::AmbiguousOutput))
    );
    assert_eq!(document.revision(), revision);
    assert!(
        document
            .large_state()
            .expect("large")
            .source
            .overlay
            .is_empty()
    );
}

#[test]
fn unencodable_text_is_rejected() {
    let fixture = Fixture::new();
    let configuration = CsvConfiguration {
        encoding: Encoding::Windows1252,
        ..CsvConfiguration::default()
    };
    let mut document = open_with_configuration(&fixture.csv(30), Some(configuration));

    assert_eq!(
        document.set_large_cell("日本", 0, 0),
        Err(CsvError::UnencodableText)
    );
    assert_eq!(
        document.notice(),
        Some(DocumentNotice::EditRejected(CsvError::UnencodableText))
    );
    assert!(
        document
            .large_state()
            .expect("large")
            .source
            .overlay
            .is_empty()
    );
}

#[test]
fn setting_the_same_text_records_nothing() {
    let fixture = Fixture::new();
    let mut document = open_without_header(&fixture.csv(30));
    let revision = document.revision();

    assert_eq!(document.set_large_cell("r0", 0, 0), Ok(()));

    assert_eq!(document.revision(), revision);
    assert_eq!(document.notice(), None);
    assert!(
        document
            .large_state()
            .expect("large")
            .source
            .overlay
            .is_empty()
    );
}

#[test]
fn a_cell_edit_is_refused_while_a_reread_is_in_flight() {
    let fixture = Fixture::new();
    let mut document = open_without_header(&fixture.csv(30));
    let revision = document.revision();

    assert!(document.begin_reread());
    assert_eq!(
        document.set_large_cell("X", 0, 0),
        Err(CsvError::RowNotEditable)
    );
    document.end_reread();

    assert_eq!(document.revision(), revision);
    assert!(
        document
            .large_state()
            .expect("large")
            .source
            .overlay
            .is_empty()
    );
}

#[test]
fn apply_parsing_configuration_routes_to_the_large_reopen() {
    let fixture = Fixture::new();
    let mut document = open_without_header(&fixture.csv(50));
    document.set_large_cell("X", 1, 1).expect("edit the cell");
    let before = source(&document);
    let mut unquoted = document.configuration().clone();
    unquoted.quote_character = String::new();
    unquoted.escape_character = String::new();

    assert_eq!(
        document.apply_parsing_configuration(unquoted.clone(), false, &StdStorage),
        Err(DocumentError::RequiresConfirmation)
    );
    assert!(Arc::ptr_eq(&before, &source(&document)));

    document
        .apply_parsing_configuration(unquoted, true, &StdStorage)
        .expect("force the reparse");
    finish_indexing(&mut document);

    assert_eq!(document.configuration().quote_character, "");
    assert!(!Arc::ptr_eq(&before, &source(&document)));
    assert_eq!(cell(&document, 1, 2), "\"q 1\"");
    assert!(
        document
            .large_state()
            .expect("large")
            .source
            .overlay
            .is_empty()
    );
    assert!(!document.can_undo());
}

#[test]
fn reopen_to_utf16_is_refused_and_changes_nothing() {
    let fixture = Fixture::new();
    let mut document = open_without_header(&fixture.csv(30));
    let before = source(&document);
    let mut utf16 = document.configuration().clone();
    utf16.encoding = Encoding::Utf16Le;

    assert_eq!(
        document.reopen_large(utf16, true),
        Err(DocumentError::Csv(CsvError::UnsupportedLargeFileEncoding))
    );

    assert!(Arc::ptr_eq(&before, &source(&document)));
    assert_eq!(document.configuration().encoding, Encoding::Utf8);
    assert_eq!(document.large_status(), LargeStatus::Ready);
}

#[test]
fn reopen_drops_the_overlay_and_undo() {
    let fixture = Fixture::new();
    let mut document = open_without_header(&fixture.csv(30));
    document.set_large_cell("X", 1, 1).expect("edit the cell");
    let configuration = document.configuration().clone();

    document
        .reopen_large(configuration, true)
        .expect("reopen the large file");
    finish_indexing(&mut document);

    assert!(
        document
            .large_state()
            .expect("large")
            .source
            .overlay
            .is_empty()
    );
    assert_eq!(cell(&document, 1, 1), "v1");
    assert!(!document.can_undo());
    assert!(!document.has_unsaved_changes());
    assert_eq!(document.large_status(), LargeStatus::Ready);
}

#[test]
fn revert_to_saved_routes_to_the_large_revert() {
    let fixture = Fixture::new();
    let mut document = open_without_header(&fixture.csv(30));
    document.set_large_cell("X", 1, 1).expect("edit the cell");
    assert_eq!(plan_revert(&document), RevertDecision::NeedsConfirmation);

    document
        .revert_to_saved(&StdStorage)
        .expect("revert the large document");

    assert!(
        document
            .large_state()
            .expect("large")
            .source
            .overlay
            .is_empty()
    );
    assert_eq!(cell(&document, 1, 1), "v1");
    assert!(!document.can_undo());
    assert_eq!(plan_revert(&document), RevertDecision::Revert);
}

#[test]
fn header_toggle_is_refused_while_the_file_is_stale() {
    let fixture = Fixture::new();
    let path = fixture.csv(50);
    let mut document = open_without_header(&path);
    fixture.append_row(50);
    document.refresh_large_staleness();
    assert_eq!(document.phase(), &DocumentPhase::Ready);

    document.set_large_header_enabled(true);

    assert!(!document.configuration().first_row_as_header);
    assert_eq!(
        document.large_status(),
        LargeStatus::Stale { appended: true }
    );
}
