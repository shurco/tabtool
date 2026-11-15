//! Port of `RevertCoordinatorTests.swift` decisions plus the transactional
//! revert invariants: one undo action, the current configuration, and an
//! untouched file.

use std::cell::{Cell, RefCell};
use std::io;
use std::path::Path;

use tabtool_core::{CsvConfiguration, CsvError};
use tabtool_document::{Document, DocumentError, RevertDecision, Storage, UndoAction, plan_revert};

/// In-memory storage that counts reads and writes and can serve edited bytes.
struct MemoryStorage {
    bytes: RefCell<Vec<u8>>,
    reads: Cell<usize>,
    writes: Cell<usize>,
    fail_reads: Cell<bool>,
}

impl MemoryStorage {
    fn new(bytes: &[u8]) -> Self {
        Self {
            bytes: RefCell::new(bytes.to_vec()),
            reads: Cell::new(0),
            writes: Cell::new(0),
            fail_reads: Cell::new(false),
        }
    }

    fn set_bytes(&self, bytes: &[u8]) {
        self.bytes.replace(bytes.to_vec());
    }
}

impl Storage for MemoryStorage {
    fn read(&self, _path: &Path) -> io::Result<Vec<u8>> {
        self.reads.set(self.reads.get() + 1);
        if self.fail_reads.get() {
            return Err(io::Error::other("the file is unavailable"));
        }
        Ok(self.bytes.borrow().clone())
    }

    fn write_atomic(&self, _path: &Path, data: &[u8]) -> io::Result<()> {
        self.writes.set(self.writes.get() + 1);
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

#[test]
fn clean_document_reverts_without_confirmation() {
    let storage = MemoryStorage::new(b"a,b\nc,d\n");
    let mut document = loaded(&storage, CsvConfiguration::default());

    assert_eq!(plan_revert(&document), RevertDecision::Revert);

    document.revert_to_saved(&storage).expect("revert the file");

    assert_eq!(document.table().expect("table").cell(0, 0), "a");
    assert_eq!(storage.reads.get(), 2);
    assert_eq!(storage.writes.get(), 0);
    assert!(!document.is_dirty());
}

#[test]
fn dirty_revert_can_be_cancelled_or_confirmed() {
    let storage = MemoryStorage::new(b"a,b\nc,d\n");
    let mut document = loaded(&storage, CsvConfiguration::default());
    document.set_cell(0, 0, "edited");
    assert_eq!(plan_revert(&document), RevertDecision::NeedsConfirmation);

    // The UI cancels the confirmation dialog: nothing changes.
    assert_eq!(document.table().expect("table").cell(0, 0), "edited");

    // The UI confirms, then the document performs the re-read.
    document
        .revert_to_saved(&storage)
        .expect("revert after confirmation");

    assert_eq!(document.table().expect("table").cell(0, 0), "a");
    assert!(!document.is_dirty());
    assert_eq!(document.undo_action(), Some(UndoAction::RevertToSaved));

    assert!(document.undo());
    assert_eq!(document.table().expect("table").cell(0, 0), "edited");
    assert!(document.is_dirty());
}

#[test]
fn revert_requires_a_file_backed_document() {
    let configuration = CsvConfiguration::default();
    let mut document = Document::new(configuration.clone());
    document
        .load_from_bytes(b"a,b\n", None, Some(configuration), false)
        .expect("load byte-only content");

    assert_eq!(plan_revert(&document), RevertDecision::Unavailable);
    assert_eq!(
        document.revert_to_saved(&MemoryStorage::new(b"")),
        Err(DocumentError::NotReady)
    );
}

#[test]
fn revert_uses_the_current_configuration() {
    let configuration = CsvConfiguration {
        column_separator: ";".to_string(),
        ..CsvConfiguration::default()
    };
    let storage = MemoryStorage::new(b"a;b\n1;2\n");
    let mut document = loaded(&storage, configuration.clone());
    storage.set_bytes(b"x;y\n3;4\n");

    document
        .revert_to_saved(&storage)
        .expect("revert with the current separator");

    let table = document.table().expect("table");
    assert_eq!(table.columns.len(), 2);
    assert_eq!(table.cell(0, 0), "x");
    assert_eq!(table.cell(0, 1), "y");
    assert_eq!(document.configuration(), &configuration);
}

#[test]
fn revert_leaves_the_file_untouched() {
    let storage = MemoryStorage::new(b"a\n");
    let mut document = loaded(&storage, CsvConfiguration::default());
    document.set_cell(0, 0, "edited");

    document.revert_to_saved(&storage).expect("revert the file");

    assert_eq!(storage.writes.get(), 0);
    assert_eq!(&*storage.bytes.borrow(), b"a\n");
}

#[test]
fn revert_registers_one_undo_action() {
    let storage = MemoryStorage::new(b"a\n");
    let mut document = loaded(&storage, CsvConfiguration::default());
    document.set_cell(0, 0, "edited");

    document.revert_to_saved(&storage).expect("revert the file");
    assert_eq!(document.undo_action(), Some(UndoAction::RevertToSaved));

    assert!(document.undo());
    assert_eq!(document.table().expect("table").cell(0, 0), "edited");
    assert_eq!(document.undo_action(), Some(UndoAction::EditCell));
}

#[test]
fn a_read_failure_keeps_the_previous_state_and_records_no_undo() {
    let storage = MemoryStorage::new(b"a\n");
    let mut document = loaded(&storage, CsvConfiguration::default());
    let revision = document.revision();
    storage.fail_reads.set(true);

    let error = document
        .revert_to_saved(&storage)
        .expect_err("the file cannot be read");

    assert_eq!(error, DocumentError::CannotReadFile);
    assert_eq!(document.table().expect("table").cell(0, 0), "a");
    assert_eq!(document.revision(), revision);
    assert!(!document.can_undo());
}

#[test]
fn a_parse_failure_keeps_the_previous_state_and_records_no_undo() {
    let storage = MemoryStorage::new(b"a\n");
    let mut document = loaded(&storage, CsvConfiguration::default());
    let revision = document.revision();
    storage.set_bytes(b"a,\"b\n");

    let error = document
        .revert_to_saved(&storage)
        .expect_err("the unterminated quote fails");

    assert_eq!(error, DocumentError::Csv(CsvError::MalformedQuotedField));
    assert_eq!(document.table().expect("table").cell(0, 0), "a");
    assert_eq!(document.revision(), revision);
    assert!(!document.can_undo());
}

#[test]
fn an_overlapping_reread_rejects_revert() {
    let storage = MemoryStorage::new(b"a\n");
    let mut document = loaded(&storage, CsvConfiguration::default());

    assert!(document.begin_reread());
    assert_eq!(
        document.revert_to_saved(&storage),
        Err(DocumentError::RequiresConfirmation)
    );
    assert_eq!(document.table().expect("table").cell(0, 0), "a");

    document.end_reread();
    document
        .revert_to_saved(&storage)
        .expect("revert after the overlap ends");
}
