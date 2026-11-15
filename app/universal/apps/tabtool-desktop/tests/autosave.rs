//! Autosave timing, skips and blocked-write reporting.
//!
//! The clock is injected: every case passes the `Instant` the scheduler should
//! believe, so no test sleeps.

mod support;

use std::time::{Duration, Instant};

use tabtool_core::{CsvConfiguration, CsvError};
use tabtool_document::{AUTOSAVE_DELAY, Autosave, AutosaveOutcome, DocumentError};

use support::MemoryStorage;

#[test]
fn a_change_is_saved_when_the_debounce_elapses() {
    let storage = MemoryStorage::new(b"a,b\n1,2\n");
    let mut document = support::loaded(&storage, CsvConfiguration::default());
    let mut autosave = Autosave::new(&document);
    let start = Instant::now();

    document.set_cell(0, 0, "edited");

    assert_eq!(
        autosave.tick(start, &mut document, &storage),
        AutosaveOutcome::Waiting {
            remaining: AUTOSAVE_DELAY
        }
    );
    assert_eq!(
        autosave.tick(
            start + AUTOSAVE_DELAY - Duration::from_millis(1),
            &mut document,
            &storage
        ),
        AutosaveOutcome::Waiting {
            remaining: Duration::from_millis(1)
        }
    );
    assert_eq!(storage.writes(), 0, "nothing is written before the delay");
    assert!(document.is_dirty());

    assert_eq!(
        autosave.tick(start + AUTOSAVE_DELAY, &mut document, &storage),
        AutosaveOutcome::Saved
    );
    assert_eq!(storage.writes(), 1);
    assert!(!document.is_dirty());
    assert_eq!(
        autosave.tick(
            start + AUTOSAVE_DELAY + Duration::from_secs(5),
            &mut document,
            &storage
        ),
        AutosaveOutcome::Idle
    );
}

#[test]
fn another_change_restarts_the_debounce() {
    let storage = MemoryStorage::new(b"a,b\n1,2\n");
    let mut document = support::loaded(&storage, CsvConfiguration::default());
    let mut autosave = Autosave::new(&document);
    let start = Instant::now();

    document.set_cell(0, 0, "first");
    assert!(matches!(
        autosave.tick(start, &mut document, &storage),
        AutosaveOutcome::Waiting { .. }
    ));

    let second = start + Duration::from_millis(400);
    document.set_cell(0, 1, "second");
    assert!(matches!(
        autosave.tick(second, &mut document, &storage),
        AutosaveOutcome::Waiting { .. }
    ));

    assert_eq!(
        autosave.tick(start + AUTOSAVE_DELAY, &mut document, &storage),
        AutosaveOutcome::Waiting {
            remaining: Duration::from_millis(400)
        }
    );
    assert_eq!(storage.writes(), 0);

    assert_eq!(
        autosave.tick(second + AUTOSAVE_DELAY, &mut document, &storage),
        AutosaveOutcome::Saved
    );
    assert_eq!(storage.writes(), 1);
}

#[test]
fn an_untouched_document_is_idle() {
    let storage = MemoryStorage::new(b"a,b\n1,2\n");
    let mut document = support::loaded(&storage, CsvConfiguration::default());
    let mut autosave = Autosave::new(&document);

    assert_eq!(
        autosave.tick(
            Instant::now() + Duration::from_secs(60),
            &mut document,
            &storage
        ),
        AutosaveOutcome::Idle
    );
    assert_eq!(storage.writes(), 0);
    assert!(autosave.deadline().is_none());
}

#[test]
fn untitled_documents_are_never_autosaved() {
    let storage = MemoryStorage::new(b"");
    let mut document = support::untitled(CsvConfiguration::default());
    let mut autosave = Autosave::new(&document);
    let start = Instant::now();

    document.set_cell(0, 0, "edited");
    assert_eq!(
        autosave.tick(start + AUTOSAVE_DELAY, &mut document, &storage),
        AutosaveOutcome::Untitled
    );
    assert_eq!(storage.writes(), 0);
    assert!(document.is_dirty());
}

#[test]
fn large_documents_are_never_autosaved() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let path = directory.path().join("big.csv");
    std::fs::write(&path, b"a,b\n1,2\n").expect("fixture file is written");

    let mut document = tabtool_document::Document::new(CsvConfiguration::default());
    document
        .open_large(&path, None, false, 0)
        .expect("open the fixture large");
    assert!(document.is_large_mode());

    let storage = tabtool_document::StdStorage;
    let mut autosave = Autosave::new(&document);

    assert_eq!(
        autosave.tick(
            Instant::now() + Duration::from_secs(60),
            &mut document,
            &storage
        ),
        AutosaveOutcome::Large
    );
}

#[test]
fn a_blocked_write_gate_keeps_the_document_dirty() {
    let configuration = CsvConfiguration {
        quote_character: String::new(),
        ..CsvConfiguration::default()
    };
    let storage = MemoryStorage::new(b"a,b\n1,2\n");
    let mut document = support::loaded(&storage, configuration);
    let mut autosave = Autosave::new(&document);
    let start = Instant::now();

    document.set_cell(0, 0, "x,y");
    assert!(matches!(
        autosave.tick(start, &mut document, &storage),
        AutosaveOutcome::Waiting { .. }
    ));
    assert_eq!(
        autosave.tick(start + AUTOSAVE_DELAY, &mut document, &storage),
        AutosaveOutcome::Blocked(DocumentError::Csv(CsvError::AmbiguousOutput))
    );
    assert_eq!(storage.writes(), 0, "the gate rejects the write");
    assert!(document.is_dirty());
    assert_eq!(
        document.table().expect("table").cell(0, 0),
        "x,y",
        "the edit is preserved"
    );
}

#[test]
fn a_failed_storage_write_keeps_the_edit_and_reports_the_failure() {
    let storage = MemoryStorage::new(b"a,b\n1,2\n");
    let mut document = support::loaded(&storage, CsvConfiguration::default());
    let mut autosave = Autosave::new(&document);
    let start = Instant::now();

    document.set_cell(0, 0, "edited");
    storage.fail_writes(true);

    assert!(matches!(
        autosave.tick(start, &mut document, &storage),
        AutosaveOutcome::Waiting { .. }
    ));
    assert_eq!(
        autosave.tick(start + AUTOSAVE_DELAY, &mut document, &storage),
        AutosaveOutcome::Blocked(DocumentError::CannotWriteFile)
    );
    assert!(document.is_dirty());
    assert_eq!(document.table().expect("table").cell(0, 0), "edited");
}

#[test]
fn the_deadline_is_set_from_the_observed_change() {
    let storage = MemoryStorage::new(b"a,b\n1,2\n");
    let mut document = support::loaded(&storage, CsvConfiguration::default());
    let mut autosave = Autosave::new(&document);
    let change = Instant::now();

    document.set_cell(0, 0, "edited");
    assert!(matches!(
        autosave.tick(change, &mut document, &storage),
        AutosaveOutcome::Waiting { .. }
    ));

    assert_eq!(autosave.deadline(), Some(change + AUTOSAVE_DELAY));
}
