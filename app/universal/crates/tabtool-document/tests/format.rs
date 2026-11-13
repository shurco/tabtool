//! Port of `CSVDocumentFormatTests.swift` plus the revision-guard and
//! untitled re-read cases for TabTool's Rust document.
//!
//! Parsing changes re-read the file through [`Storage`]; decimal and header
//! changes transform the in-memory table with one undo step each.

use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use tabtool_core::{CsvConfiguration, CsvError, CsvTable, CsvWriteIssue, CsvWriter};
use tabtool_document::{
    Document, DocumentError, DocumentPhase, ParsedTable, StdStorage, Storage, UndoAction,
};

fn temp_file(contents: &[u8]) -> (tempfile::TempDir, PathBuf) {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let path = directory.path().join("document.csv");
    std::fs::write(&path, contents).expect("a writable file");
    (directory, path)
}

fn loaded_from_path(path: &Path, configuration: CsvConfiguration) -> Document {
    let mut document = Document::new(configuration.clone());
    document
        .load_from_path(path, &StdStorage, Some(configuration), false)
        .expect("load the fixture");
    document
}

fn loaded(bytes: &[u8], configuration: CsvConfiguration) -> Document {
    let mut document = Document::new(configuration.clone());
    document
        .load_from_bytes(bytes, None, Some(configuration), false)
        .expect("load the fixture");
    document
}

/// Storage that serves fixed bytes and refuses writes.
struct FixedStorage {
    bytes: Vec<u8>,
}

impl Storage for FixedStorage {
    fn read(&self, _path: &Path) -> io::Result<Vec<u8>> {
        Ok(self.bytes.clone())
    }

    fn write_atomic(&self, _path: &Path, _data: &[u8]) -> io::Result<()> {
        panic!("a re-read must not write");
    }
}

/// Storage that panics when read, proving an untitled document never rereads.
struct NoReadStorage;

impl Storage for NoReadStorage {
    fn read(&self, _path: &Path) -> io::Result<Vec<u8>> {
        panic!("an untitled document has nothing to re-read");
    }

    fn write_atomic(&self, _path: &Path, _data: &[u8]) -> io::Result<()> {
        panic!("a re-read must not write");
    }
}

#[test]
fn decimal_change_converts_numeric_cells_and_undoes() {
    let configuration = CsvConfiguration {
        column_separator: ";".to_string(),
        decimal_mark: ",".to_string(),
        ..CsvConfiguration::default()
    };
    let mut document = loaded(b"1,50\nhello\n", configuration);

    document.set_decimal_mark(".");

    let table = document.table().expect("table");
    assert_eq!(table.cell(0, 0), "1.50");
    assert_eq!(table.cell(1, 0), "hello");
    assert_eq!(document.configuration().decimal_mark, ".");
    assert_eq!(document.undo_action(), Some(UndoAction::ChangeFormat));

    assert!(document.undo());
    assert_eq!(document.table().expect("table").cell(0, 0), "1,50");
    assert_eq!(document.configuration().decimal_mark, ",");
    assert!(!document.can_undo());
}

#[test]
fn new_document_decimal_and_header_transform_typed_table() {
    let configuration = CsvConfiguration::default();
    let mut document = Document::new(configuration.clone());
    document
        .new_document(configuration)
        .expect("replace with a new document");

    document.set_cell(0, 0, "1.50");
    document.set_decimal_mark(",");
    assert_eq!(document.table().expect("table").cell(0, 0), "1,50");

    document.set_header_enabled(true);
    let table = document.table().expect("table");
    assert_eq!(
        table.headers(),
        Some(vec!["1,50".to_string(), String::new(), String::new()])
    );
    assert_eq!(table.row_count(), 0);
    assert_eq!(document.undo_action(), Some(UndoAction::ToggleHeader));
}

#[test]
fn header_toggle_moves_first_row_and_undoes() {
    let configuration = CsvConfiguration {
        first_row_as_header: true,
        ..CsvConfiguration::default()
    };
    let mut document = loaded(b"h1,h2\na,b\n", configuration);
    assert_eq!(
        document.table().expect("table").headers(),
        Some(vec!["h1".to_string(), "h2".to_string()])
    );

    document.set_header_enabled(false);

    let table = document.table().expect("table");
    assert_eq!(table.headers(), None);
    assert_eq!(table.cell(0, 0), "h1");
    assert_eq!(table.cell(0, 1), "h2");
    assert_eq!(table.cell(1, 0), "a");
    assert_eq!(document.undo_action(), Some(UndoAction::ToggleHeader));

    assert!(document.undo());
    assert_eq!(
        document.table().expect("table").headers(),
        Some(vec!["h1".to_string(), "h2".to_string()])
    );
    assert_eq!(document.table().expect("table").row_count(), 1);
}

#[test]
fn dirty_reparse_requires_confirmation() {
    let (_directory, path) = temp_file(b"a,b\nc,d\n");
    let mut document = loaded_from_path(&path, CsvConfiguration::default());
    document.set_cell(0, 0, "x");
    assert!(document.has_unsaved_changes());

    let error = document
        .apply_parsing_configuration(
            CsvConfiguration {
                column_separator: ";".to_string(),
                ..CsvConfiguration::default()
            },
            false,
            &StdStorage,
        )
        .expect_err("a dirty re-read must be confirmed first");

    assert_eq!(error, DocumentError::RequiresConfirmation);
    assert_eq!(document.table().expect("table").cell(0, 0), "x");
    assert_eq!(document.configuration().column_separator, ",");
    assert_eq!(document.undo_action(), Some(UndoAction::EditCell));
}

#[test]
fn forced_reparse_replaces_table_and_undoes() {
    let (_directory, path) = temp_file(b"a,b\nc,d\n");
    let mut document = loaded_from_path(&path, CsvConfiguration::default());
    document.set_cell(0, 0, "x");

    document
        .apply_parsing_configuration(
            CsvConfiguration {
                column_separator: ";".to_string(),
                ..CsvConfiguration::default()
            },
            true,
            &StdStorage,
        )
        .expect("a forced re-read");

    let table = document.table().expect("table");
    assert_eq!(table.columns.len(), 1);
    assert_eq!(table.cell(0, 0), "a,b");
    assert_eq!(document.configuration().column_separator, ";");
    assert!(!document.is_dirty());
    assert!(!document.has_unsaved_changes());
    assert_eq!(document.undo_action(), Some(UndoAction::ChangeFormat));

    assert!(document.undo());
    assert_eq!(document.table().expect("table").cell(0, 0), "x");
    assert_eq!(document.configuration().column_separator, ",");
}

#[test]
fn failed_reparse_keeps_table_and_configuration_without_undo() {
    let (_directory, path) = temp_file(b"a,\"b\n");
    let configuration = CsvConfiguration {
        quote_character: String::new(),
        ..CsvConfiguration::default()
    };
    let mut document = loaded_from_path(&path, configuration.clone());
    let table = Arc::clone(document.table().expect("table"));
    let revision = document.revision();

    let error = document
        .apply_parsing_configuration(CsvConfiguration::default(), true, &StdStorage)
        .expect_err("the unterminated quote fails");

    assert_eq!(error, DocumentError::Csv(CsvError::MalformedQuotedField));
    assert!(Arc::ptr_eq(document.table().expect("table"), &table));
    assert_eq!(document.configuration(), &configuration);
    assert_eq!(document.revision(), revision);
    assert!(!document.can_undo());
}

#[test]
fn missing_file_reparse_reports_cannot_read_file() {
    let (_directory, path) = temp_file(b"a\n");
    let mut document = loaded_from_path(&path, CsvConfiguration::default());
    std::fs::remove_file(&path).expect("remove the file");
    let revision = document.revision();

    let error = document
        .apply_parsing_configuration(
            CsvConfiguration {
                column_separator: ";".to_string(),
                ..CsvConfiguration::default()
            },
            true,
            &StdStorage,
        )
        .expect_err("the file is gone");

    assert_eq!(error, DocumentError::CannotReadFile);
    assert_eq!(document.table().expect("table").cell(0, 0), "a");
    assert_eq!(document.configuration().column_separator, ",");
    assert_eq!(document.revision(), revision);
    assert!(!document.can_undo());
}

#[test]
fn untitled_configuration_change_cannot_reread_and_keeps_the_table() {
    let mut document = loaded(b"a,b\n", CsvConfiguration::default());
    let table = Arc::clone(document.table().expect("table"));

    document
        .apply_parsing_configuration(
            CsvConfiguration {
                column_separator: ";".to_string(),
                ..CsvConfiguration::default()
            },
            false,
            &NoReadStorage,
        )
        .expect("an untitled document only changes its configuration");

    assert!(Arc::ptr_eq(document.table().expect("table"), &table));
    assert_eq!(document.configuration().column_separator, ";");
    assert_eq!(document.undo_action(), Some(UndoAction::ChangeFormat));
    assert!(document.has_unsaved_changes());

    assert!(document.undo());
    assert_eq!(document.configuration().column_separator, ",");
}

#[test]
fn untitled_configuration_change_recomputes_the_write_issue() {
    let mut document = loaded(b"a;b\n", CsvConfiguration::default());
    assert_eq!(document.phase(), &DocumentPhase::Ready);

    document
        .apply_parsing_configuration(
            CsvConfiguration {
                column_separator: ";".to_string(),
                quote_character: String::new(),
                ..CsvConfiguration::default()
            },
            false,
            &NoReadStorage,
        )
        .expect("the configuration changes in memory");

    assert_eq!(
        document.phase(),
        &DocumentPhase::NeedsAttention(CsvWriteIssue::AmbiguousOutput)
    );

    assert!(document.undo());
    assert_eq!(document.phase(), &DocumentPhase::Ready);
}

#[test]
fn reparse_reads_the_bytes_through_storage() {
    let configuration = CsvConfiguration::default();
    let mut document = Document::new(configuration.clone());
    document
        .load_from_bytes(
            b"a,b\n",
            Some(Path::new("/virtual/document.csv")),
            Some(configuration),
            false,
        )
        .expect("load the initial bytes");
    let storage = FixedStorage {
        bytes: b"x;y\n".to_vec(),
    };

    document
        .apply_parsing_configuration(
            CsvConfiguration {
                column_separator: ";".to_string(),
                ..CsvConfiguration::default()
            },
            false,
            &storage,
        )
        .expect("the re-read uses the storage bytes");

    let table = document.table().expect("table");
    assert_eq!(table.columns.len(), 2);
    assert_eq!(table.cell(0, 0), "x");
    assert_eq!(table.cell(0, 1), "y");
}

#[test]
fn reparse_updates_the_utf8_bom_and_keeps_older_snapshots() {
    let mut input = vec![0xEF, 0xBB, 0xBF];
    input.extend_from_slice(b"a\n");
    let (_directory, path) = temp_file(&input);
    let mut document = loaded_from_path(&path, CsvConfiguration::default());
    let snapshot = document.snapshot().expect("snapshot");
    assert!(snapshot.includes_utf8_bom);

    std::fs::write(&path, b"a\n").expect("rewrite without the mark");
    document
        .apply_parsing_configuration(CsvConfiguration::default(), true, &StdStorage)
        .expect("the same configuration still re-reads");

    assert!(!document.snapshot().expect("snapshot").includes_utf8_bom);
    assert!(snapshot.includes_utf8_bom);

    let mut writer = CsvWriter::new(&snapshot.table, None, &snapshot.configuration);
    writer.include_utf8_bom = snapshot.includes_utf8_bom;
    assert!(
        writer
            .data()
            .expect("encode the old snapshot")
            .starts_with(&[0xEF, 0xBB, 0xBF])
    );
}

#[test]
fn detect_format_from_disk_reads_without_applying() {
    let (_directory, path) = temp_file(b"a;b\n1,5;2,5\n");
    let document = loaded_from_path(&path, CsvConfiguration::default());
    let revision = document.revision();

    let detected = document
        .detect_format_from_disk(&StdStorage, false)
        .expect("detect the disk format");

    assert_eq!(detected.column_separator, ";");
    assert_eq!(detected.decimal_mark, ",");
    assert_eq!(document.configuration().column_separator, ",");
    assert_eq!(document.revision(), revision);
}

#[test]
fn reparse_recovers_a_read_failed_document_without_undo() {
    let (_directory, path) = temp_file(b"a,\"b\n");
    let mut document = Document::new(CsvConfiguration::default());
    assert_eq!(
        document.load_from_path(&path, &StdStorage, Some(CsvConfiguration::default()), false),
        Err(DocumentError::NotReady)
    );
    assert!(matches!(document.phase(), DocumentPhase::ReadFailed(_)));
    assert!(document.table().is_none());

    document
        .apply_parsing_configuration(
            CsvConfiguration {
                quote_character: String::new(),
                ..CsvConfiguration::default()
            },
            false,
            &StdStorage,
        )
        .expect("the relaxed quote configuration parses");

    assert_eq!(document.phase(), &DocumentPhase::Ready);
    assert!(document.table().is_some());
    assert!(!document.can_undo());
}

#[test]
fn detect_format_on_an_untitled_document_returns_the_current_configuration() {
    let configuration = CsvConfiguration {
        column_separator: ";".to_string(),
        ..CsvConfiguration::default()
    };
    let document = loaded(b"a;b\n", configuration.clone());

    let detected = document
        .detect_format_from_disk(&NoReadStorage, true)
        .expect("no path means no read");

    assert_eq!(detected, configuration);
}

#[test]
fn overlapping_reread_is_rejected() {
    let (_directory, path) = temp_file(b"a\n");
    let mut document = loaded_from_path(&path, CsvConfiguration::default());

    assert!(document.begin_reread());
    assert!(!document.begin_reread());

    let error = document
        .apply_parsing_configuration(
            CsvConfiguration {
                column_separator: ";".to_string(),
                ..CsvConfiguration::default()
            },
            true,
            &StdStorage,
        )
        .expect_err("one re-read at a time");

    assert_eq!(error, DocumentError::RequiresConfirmation);
    assert_eq!(document.table().expect("table").cell(0, 0), "a");
    assert_eq!(document.configuration().column_separator, ",");

    document.end_reread();
    document
        .apply_parsing_configuration(
            CsvConfiguration {
                column_separator: ";".to_string(),
                ..CsvConfiguration::default()
            },
            true,
            &StdStorage,
        )
        .expect("the re-read after the overlap ends");
}

#[test]
fn a_stale_candidate_is_rejected_after_a_mutation() {
    let mut document = loaded(b"a\n", CsvConfiguration::default());
    let revision = document.revision();
    let candidate = ParsedTable {
        table: CsvTable::from_rows(vec![vec!["z".to_string()]], None, 1),
        configuration: CsvConfiguration::default(),
        had_utf8_bom: false,
        expected_revision: revision,
    };
    document.set_cell(0, 0, "x");

    document.apply_reparsed_candidate(candidate);

    assert_eq!(document.table().expect("table").cell(0, 0), "x");
    assert_eq!(document.revision(), revision + 1);
    assert_eq!(document.undo_action(), Some(UndoAction::EditCell));
}

#[test]
fn a_candidate_at_the_current_revision_applies_with_one_undo() {
    let mut document = loaded(b"a\n", CsvConfiguration::default());
    let revision = document.revision();
    let candidate = ParsedTable {
        table: CsvTable::from_rows(vec![vec!["z".to_string()]], None, 1),
        configuration: CsvConfiguration {
            column_separator: ";".to_string(),
            ..CsvConfiguration::default()
        },
        had_utf8_bom: false,
        expected_revision: revision,
    };

    document.apply_reparsed_candidate(candidate);

    assert_eq!(document.table().expect("table").cell(0, 0), "z");
    assert_eq!(document.configuration().column_separator, ";");
    assert_eq!(document.undo_action(), Some(UndoAction::ChangeFormat));
    assert!(document.undo());
    assert_eq!(document.table().expect("table").cell(0, 0), "a");
    assert_eq!(document.configuration().column_separator, ",");
    assert!(!document.can_undo());
}

#[test]
fn has_unsaved_changes_marks_untitled_documents_and_tracks_revisions() {
    let mut untitled = Document::new(CsvConfiguration::default());
    assert!(untitled.has_unsaved_changes());
    untitled
        .new_document(CsvConfiguration::default())
        .expect("replace with a new document");
    assert!(untitled.has_unsaved_changes());

    let (_directory, path) = temp_file(b"a\n");
    let mut document = loaded_from_path(&path, CsvConfiguration::default());
    assert!(!document.has_unsaved_changes());
    document.set_cell(0, 0, "x");
    assert!(document.has_unsaved_changes());
    document
        .apply_parsing_configuration(CsvConfiguration::default(), true, &StdStorage)
        .expect("re-read the disk content");
    assert!(!document.has_unsaved_changes());
}
