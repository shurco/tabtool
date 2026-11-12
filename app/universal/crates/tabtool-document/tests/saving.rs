//! Port of the save and write-gate cases in `CSVDocumentTests.swift`,
//! `CSVDocumentFormatTests.swift` and the snapshot cache in
//! `CSVDocumentSaving.swift`, plus the atomic-write and revision invariants of
//! the Rust save path.

use std::cell::{Cell, RefCell};
use std::io;
use std::path::Path;

use tabtool_core::{CsvConfiguration, CsvError, CsvTable, CsvWriteIssue, Encoding};
use tabtool_document::{Document, DocumentError, DocumentPhase, ParsedTable, StdStorage, Storage};

/// In-memory [`Storage`] that records reads and writes and can refuse either.
struct MemoryStorage {
    bytes: RefCell<Vec<u8>>,
    writes: Cell<usize>,
    fail_reads: Cell<bool>,
    fail_writes: Cell<bool>,
}

impl MemoryStorage {
    fn new(bytes: &[u8]) -> Self {
        Self {
            bytes: RefCell::new(bytes.to_vec()),
            writes: Cell::new(0),
            fail_reads: Cell::new(false),
            fail_writes: Cell::new(false),
        }
    }

    fn bytes(&self) -> Vec<u8> {
        self.bytes.borrow().clone()
    }
}

impl Storage for MemoryStorage {
    fn read(&self, _path: &Path) -> io::Result<Vec<u8>> {
        if self.fail_reads.get() {
            return Err(io::Error::other("the file is unavailable"));
        }
        Ok(self.bytes())
    }

    fn write_atomic(&self, _path: &Path, data: &[u8]) -> io::Result<()> {
        self.writes.set(self.writes.get() + 1);
        if self.fail_writes.get() {
            return Err(io::Error::other("the disk is full"));
        }
        self.bytes.replace(data.to_vec());
        Ok(())
    }
}

fn loaded(storage: &MemoryStorage, configuration: CsvConfiguration) -> Document {
    let mut document = Document::new(configuration.clone());
    document
        .load_from_path(
            Path::new("/virtual/document.csv"),
            storage,
            Some(configuration),
            false,
        )
        .expect("load the fixture");
    document
}

fn loaded_bytes(csv: &[u8], configuration: CsvConfiguration) -> Document {
    let mut document = Document::new(configuration.clone());
    document
        .load_from_bytes(csv, None, Some(configuration), false)
        .expect("load the fixture");
    document
}

fn windows_1252() -> CsvConfiguration {
    CsvConfiguration {
        encoding: Encoding::Windows1252,
        ..CsvConfiguration::default()
    }
}

#[test]
fn untitled_documents_are_dirty_until_save_as() {
    let mut document = Document::new(CsvConfiguration::default());
    document.new_document(CsvConfiguration::default());

    assert!(document.file_path().is_none());
    assert!(document.is_dirty());
    assert!(!document.can_save());

    let directory = tempfile::tempdir().expect("a temporary directory");
    let path = directory.path().join("untitled.csv");

    document.save_as(&path, &StdStorage).expect("Save As");

    assert_eq!(document.file_path(), Some(path.as_path()));
    assert!(!document.is_dirty());
    assert!(!document.has_unsaved_changes());
    assert!(document.can_save());
}

#[test]
fn a_file_backed_document_starts_clean_and_becomes_dirty_after_an_edit() {
    let storage = MemoryStorage::new(b"a,b\n1,2\n");
    let mut document = loaded(&storage, CsvConfiguration::default());

    assert!(!document.is_dirty());

    document.set_cell(1, 1, "42");

    assert!(document.is_dirty());
    assert!(document.has_unsaved_changes());
}

#[test]
fn save_writes_the_encoded_snapshot_and_marks_the_revision_saved() {
    let storage = MemoryStorage::new(b"a,b\n1,2\n");
    let mut document = loaded(&storage, CsvConfiguration::default());
    document.set_cell(1, 1, "42");

    let expected = document.encoded_data().expect("encoded data");
    let revision = document.revision();

    document.save(&storage).expect("save the document");

    assert_eq!(storage.writes.get(), 1);
    assert_eq!(storage.bytes(), expected);
    assert_eq!(
        document.revision(),
        revision,
        "saving is not a content change"
    );
    assert!(!document.is_dirty());
    assert_eq!(document.table().expect("table").cell(1, 1), "42");
}

#[test]
fn saving_a_clean_document_rewrites_the_file() {
    let storage = MemoryStorage::new(b"a,b\n");
    let mut document = loaded(&storage, CsvConfiguration::default());

    document.save(&storage).expect("the first save");
    document.save(&storage).expect("the second save");

    assert_eq!(storage.writes.get(), 2);
    assert_eq!(
        storage.bytes(),
        document.encoded_data().expect("encoded data")
    );
}

#[test]
fn save_preserves_the_utf8_bom_of_the_loaded_file() {
    let storage = MemoryStorage::new(b"\xEF\xBB\xBFa,b\n");
    let mut document = loaded(&storage, CsvConfiguration::default());

    document.save(&storage).expect("save the document");

    assert!(storage.bytes().starts_with(&[0xEF, 0xBB, 0xBF]));
}

#[test]
fn save_without_a_path_is_not_ready() {
    let storage = MemoryStorage::new(b"");
    let mut document = Document::new(CsvConfiguration::default());
    document.new_document(CsvConfiguration::default());

    assert!(!document.can_save());
    assert_eq!(document.save(&storage), Err(DocumentError::NotReady));
    assert_eq!(storage.writes.get(), 0);
    assert!(document.is_dirty());
}

#[test]
fn save_as_writes_a_new_file_binds_the_path_and_clears_dirty() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let path = directory.path().join("saved.csv");
    let mut document = Document::new(CsvConfiguration::default());
    document.new_document(CsvConfiguration::default());
    let expected = document.encoded_data().expect("encoded data");

    document.save_as(&path, &StdStorage).expect("Save As");

    assert_eq!(std::fs::read(&path).expect("the saved file"), expected);
    assert_eq!(document.file_path(), Some(path.as_path()));
    assert!(!document.is_dirty());
}

#[test]
fn a_save_as_failure_keeps_the_previous_path_and_dirty_state() {
    let storage = MemoryStorage::new(b"a,b\nc,d\n");
    let mut document = loaded(&storage, CsvConfiguration::default());
    let original = document.file_path().map(Path::to_path_buf);
    document.set_cell(1, 0, "x");
    storage.fail_writes.set(true);

    assert!(document.is_dirty());
    assert_eq!(
        document.save_as(Path::new("/virtual/new.csv"), &storage),
        Err(DocumentError::CannotWriteFile)
    );

    assert_eq!(
        document.file_path().map(Path::to_path_buf),
        original,
        "the path changes only after a successful write"
    );
    assert!(document.is_dirty());
}

#[test]
fn a_failed_save_keeps_the_previous_file_and_the_document_dirty() {
    let storage = MemoryStorage::new(b"a,b\n1,2\n");
    let mut document = loaded(&storage, CsvConfiguration::default());
    document.set_cell(0, 0, "edited");
    let revision = document.revision();
    storage.fail_writes.set(true);

    assert_eq!(document.save(&storage), Err(DocumentError::CannotWriteFile));

    assert_eq!(storage.writes.get(), 1);
    assert_eq!(storage.bytes(), b"a,b\n1,2\n");
    assert_eq!(document.revision(), revision);
    assert!(document.is_dirty());

    // A later attempt with a working target saves the same edits.
    storage.fail_writes.set(false);
    document.save(&storage).expect("retry the save");
    assert!(!document.is_dirty());
}

#[test]
fn an_unwritable_target_leaves_the_document_dirty() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let path = directory.path().join("missing").join("document.csv");
    let mut document = loaded_bytes(b"a,b\n", CsvConfiguration::default());
    document.set_cell(0, 0, "x");

    assert_eq!(
        document.save_as(&path, &StdStorage),
        Err(DocumentError::CannotWriteFile)
    );
    assert!(!path.exists());
    assert!(document.file_path().is_none());
    assert!(document.is_dirty());
}

#[test]
fn a_loading_document_cannot_save() {
    let storage = MemoryStorage::new(b"");
    let mut document = Document::new(CsvConfiguration::default());

    assert!(!document.can_save());
    assert_eq!(document.save(&storage), Err(DocumentError::NotReady));
    assert_eq!(
        document.save_as(Path::new("/virtual/document.csv"), &storage),
        Err(DocumentError::NotReady)
    );
    assert_eq!(storage.writes.get(), 0);
}

#[test]
fn a_read_failed_document_cannot_save() {
    let storage = MemoryStorage::new(b"");
    let mut document = Document::new(CsvConfiguration::default());
    document
        .load_from_bytes(
            &[0xFF, 0xFD],
            Some(Path::new("/virtual/document.csv")),
            Some(CsvConfiguration::default()),
            false,
        )
        .expect_err("the bytes are not valid CSV");
    assert_eq!(
        document.phase(),
        &DocumentPhase::ReadFailed(CsvError::CannotDecodeData)
    );

    assert!(!document.can_save());
    assert_eq!(document.save(&storage), Err(DocumentError::NotReady));
    assert_eq!(
        document.save_as(Path::new("/virtual/new.csv"), &storage),
        Err(DocumentError::NotReady)
    );
    assert_eq!(storage.writes.get(), 0);
}

#[test]
fn an_unreadable_file_document_cannot_save() {
    let storage = MemoryStorage::new(b"");
    storage.fail_reads.set(true);
    let mut document = Document::new(CsvConfiguration::default());
    assert_eq!(
        document.load_from_path(
            Path::new("/virtual/document.csv"),
            &storage,
            Some(CsvConfiguration::default()),
            false,
        ),
        Err(DocumentError::CannotReadFile)
    );
    assert_eq!(document.phase(), &DocumentPhase::UnreadableFile);

    assert!(!document.can_save());
    assert_eq!(document.save(&storage), Err(DocumentError::NotReady));
    assert_eq!(storage.writes.get(), 0);
}

#[test]
fn unencodable_text_blocks_save_with_the_typed_error() {
    let storage = MemoryStorage::new(b"x\n");
    let mut document = loaded(&storage, windows_1252());
    document.set_cell(0, 0, "Привет");

    assert_eq!(
        document.phase(),
        &DocumentPhase::NeedsAttention(CsvWriteIssue::UnencodableText)
    );
    assert_eq!(document.save_error(), Some(CsvError::UnencodableText));
    assert!(!document.can_save());
    assert_eq!(
        document.save(&storage),
        Err(DocumentError::Csv(CsvError::UnencodableText))
    );
    assert_eq!(storage.writes.get(), 0);
    assert!(document.is_dirty());
}

#[test]
fn ambiguous_no_quote_output_blocks_save_with_the_typed_error() {
    let configuration = CsvConfiguration {
        quote_character: String::new(),
        ..CsvConfiguration::default()
    };
    let storage = MemoryStorage::new(b"a\n\n");
    let mut document = loaded(&storage, configuration);

    assert_eq!(
        document.phase(),
        &DocumentPhase::NeedsAttention(CsvWriteIssue::AmbiguousOutput)
    );
    assert_eq!(document.save_error(), Some(CsvError::AmbiguousOutput));
    assert!(!document.can_save());
    assert_eq!(
        document.save(&storage),
        Err(DocumentError::Csv(CsvError::AmbiguousOutput))
    );
    assert_eq!(
        document.save_as(Path::new("/virtual/new.csv"), &storage),
        Err(DocumentError::Csv(CsvError::AmbiguousOutput))
    );
    assert_eq!(storage.writes.get(), 0);
}

#[test]
fn undo_recomputes_the_write_issue_and_allows_saving_again() {
    let storage = MemoryStorage::new(b"x\n");
    let mut document = loaded(&storage, windows_1252());
    document.set_cell(0, 0, "Привет");
    assert_eq!(document.save_error(), Some(CsvError::UnencodableText));

    assert!(document.undo());

    assert_eq!(document.phase(), &DocumentPhase::Ready);
    assert_eq!(document.save_error(), None);
    assert_eq!(document.refresh_write_issue(), None);
    assert!(document.can_save());

    let expected = document.encoded_data().expect("encoded data");
    document.save(&storage).expect("save the restored text");
    assert_eq!(storage.bytes(), expected);
    assert!(!document.is_dirty());
}

#[test]
fn refresh_write_issue_recomputes_the_phase() {
    let storage = MemoryStorage::new(b"x\n");
    let mut document = loaded(&storage, windows_1252());

    document.set_cell(0, 0, "Привет");
    assert_eq!(
        document.refresh_write_issue(),
        Some(CsvWriteIssue::UnencodableText)
    );
    assert_eq!(
        document.phase(),
        &DocumentPhase::NeedsAttention(CsvWriteIssue::UnencodableText)
    );

    document.set_cell(0, 0, "ok");
    assert_eq!(document.refresh_write_issue(), None);
    assert_eq!(document.phase(), &DocumentPhase::Ready);
}

#[test]
fn save_does_not_change_the_table_or_configuration() {
    let configuration = CsvConfiguration {
        first_row_as_header: true,
        ..CsvConfiguration::default()
    };
    let storage = MemoryStorage::new(b"a,b\n1,2\n");
    let mut document = loaded(&storage, configuration.clone());
    document.set_cell(1, 0, "9");
    let table = document.table().expect("table").clone();

    document.save(&storage).expect("save the document");

    assert_eq!(document.configuration(), &configuration);
    assert!(std::sync::Arc::ptr_eq(
        document.table().expect("table"),
        &table
    ));
}

#[test]
fn save_after_save_as_targets_the_new_path() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let first = directory.path().join("first.csv");
    let second = directory.path().join("second.csv");
    let mut document = loaded_bytes(b"a\n", CsvConfiguration::default());
    document
        .save_as(&first, &StdStorage)
        .expect("the first path");

    document
        .save_as(&second, &StdStorage)
        .expect("the second path");
    document.set_cell(0, 0, "b");
    document.save(&StdStorage).expect("save to the new path");

    assert_eq!(document.file_path(), Some(second.as_path()));
    assert_eq!(
        std::fs::read(&second).expect("the new file"),
        b"\"b\"".to_vec()
    );
    assert_eq!(std::fs::read(&first).expect("the old file"), b"\"a\"");
}

#[test]
fn undoing_an_edit_after_a_save_keeps_the_document_dirty() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let path = directory.path().join("saved.csv");
    let mut document = loaded_bytes(b"a\n", CsvConfiguration::default());
    document
        .save_as(&path, &StdStorage)
        .expect("the initial save");
    assert!(!document.is_dirty());

    document.set_cell(0, 0, "b");
    assert!(document.is_dirty());

    assert!(document.undo());
    assert_eq!(document.table().expect("table").cell(0, 0), "a");

    // Dirty is revision-based, not a content comparison with the file.
    assert!(document.is_dirty());
    assert_eq!(std::fs::read(&path).expect("the saved file"), b"\"a\"");
}

#[test]
fn save_is_refused_while_a_reread_is_in_flight() {
    let storage = MemoryStorage::new(b"a,b\n");
    let mut document = loaded(&storage, CsvConfiguration::default());
    assert!(document.begin_reread());

    assert!(!document.can_save());
    assert_eq!(
        document.save(&storage),
        Err(DocumentError::RequiresConfirmation)
    );
    assert_eq!(
        document.save_as(Path::new("/virtual/new.csv"), &storage),
        Err(DocumentError::RequiresConfirmation)
    );
    assert_eq!(storage.writes.get(), 0);

    // Complete the public begin/apply/end cycle the UI uses.
    let configuration = CsvConfiguration::default();
    document.apply_reparsed_candidate(ParsedTable {
        table: CsvTable::from_rows(vec![vec!["x".to_string()]], None, 1),
        configuration,
        had_utf8_bom: false,
        expected_revision: document.revision(),
    });
    document.end_reread();

    assert!(document.can_save());
    document.save(&storage).expect("save after the re-read");
    assert_eq!(storage.writes.get(), 1);
}

#[test]
fn a_real_atomic_write_failure_leaves_the_target_untouched() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    // A directory is an existing target that `StdStorage` can open for
    // metadata but cannot replace atomically; the temporary file is created
    // and then the install step fails.
    let target = directory.path().join("folder");
    std::fs::create_dir(&target).expect("a directory target");
    std::fs::write(target.join("inside.txt"), b"keep").expect("target content");

    let configuration = CsvConfiguration::default();
    let mut document = Document::new(configuration.clone());
    document
        .load_from_bytes(b"a\n", Some(&target), Some(configuration), false)
        .expect("load the fixture");
    document.set_cell(0, 0, "b");

    assert_eq!(
        document.save(&StdStorage),
        Err(DocumentError::CannotWriteFile)
    );

    assert!(target.is_dir());
    assert_eq!(
        std::fs::read(target.join("inside.txt")).expect("target content"),
        b"keep"
    );
    let leftovers: Vec<_> = std::fs::read_dir(directory.path())
        .expect("the directory")
        .map(|entry| entry.expect("an entry").file_name())
        .filter(|name| name.to_string_lossy().starts_with(".tabtool-save-"))
        .collect();
    assert!(leftovers.is_empty(), "no temporary save file remains");
    assert!(document.is_dirty());
    assert_eq!(document.file_path(), Some(target.as_path()));
}

#[test]
fn can_save_requires_a_path_a_table_and_a_writable_phase() {
    let storage = MemoryStorage::new(b"a,b\n");
    let mut document = loaded(&storage, CsvConfiguration::default());
    assert!(document.can_save());

    document.set_file_path(None);
    assert!(!document.can_save(), "an untitled document needs Save As");
}
