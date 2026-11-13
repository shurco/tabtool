//! Task 9a/9b mechanics: the save job, its gates, progress, cancellation and
//! the restore of the original mapping. The document-level behavior of
//! `CSVDocumentLargeSavingTests.swift` lives in `large_save.rs`.

mod support;

use std::sync::Arc;
use std::time::{Duration, Instant};

use support::{
    Fixture, inode, open_indexing, open_without_header, source, start_error, wait_until_complete,
};
use tabtool_core::{CsvConfiguration, CsvError};
use tabtool_document::{Document, DocumentNotice, LargeSaveJobOutcome, LargeStatus, UndoAction};
use tabtool_large::LargeFileSaveError;

#[test]
fn start_is_refused_while_the_index_is_incomplete() {
    let fixture = Fixture::new();
    let path = fixture.large_csv(500_000);
    let mut document = open_indexing(&path);
    assert!(matches!(
        document.large_status(),
        LargeStatus::Indexing(fraction) if fraction < 1.0
    ));

    let error = start_error(&mut document, None);

    assert!(matches!(error, LargeFileSaveError::IndexingNotFinished));
    assert!(matches!(document.large_status(), LargeStatus::Indexing(_)));
    assert!(document.is_large_mode());
}

#[test]
fn a_stale_file_is_refused() {
    let fixture = Fixture::new();
    let path = fixture.csv(300);
    let mut document = open_without_header(&path);
    std::fs::write(&path, b"r0").expect("truncate the fixture");
    document.refresh_large_staleness();
    assert_eq!(
        document.large_status(),
        LargeStatus::Stale { appended: false }
    );

    let error = start_error(&mut document, Some(&fixture.copy_path()));

    assert!(matches!(error, LargeFileSaveError::SourceChanged));
    assert_eq!(
        document.large_status(),
        LargeStatus::Stale { appended: false }
    );
    assert!(document.is_large_mode());
}

#[test]
fn a_non_large_document_is_refused() {
    let mut document = Document::new(CsvConfiguration::default());

    let error = start_error(&mut document, None);

    assert!(matches!(error, LargeFileSaveError::IndexingNotFinished));
    assert_eq!(document.large_status(), LargeStatus::None);
    assert!(!document.is_large_mode());
}

#[test]
fn progress_is_reported_only_while_saving() {
    let fixture = Fixture::new();
    let path = fixture.csv(300);
    let mut document = open_without_header(&path);
    document.set_large_cell("X", 2, 1).expect("edit the cell");
    let copy = fixture.copy_path();

    let _job = document
        .start_large_save(Some(&copy))
        .expect("start the save as");
    assert!(matches!(document.large_status(), LargeStatus::Saving(_)));
    match document.large_save_progress() {
        Some(progress) => assert!((0.0..=1.0).contains(&progress)),
        None => panic!("a running save must report progress"),
    }

    wait_until_complete(&document);
    assert_eq!(document.large_save_progress(), Some(1.0));
    assert!(matches!(
        document.large_status(),
        LargeStatus::Saving(progress) if progress == 1.0
    ));

    document
        .wait_for_large_save()
        .expect("apply the save outcome");
    assert_eq!(document.large_save_progress(), None);
    assert_eq!(document.large_status(), LargeStatus::Ready);
}

#[test]
fn a_second_start_is_rejected_while_saving() {
    let fixture = Fixture::new();
    let path = fixture.csv(300);
    let mut document = open_without_header(&path);
    document.set_large_cell("X", 2, 1).expect("edit the cell");
    let copy = fixture.copy_path();
    let _job = document
        .start_large_save(Some(&copy))
        .expect("start the first save");

    let error = start_error(&mut document, Some(&copy));

    assert!(matches!(error, LargeFileSaveError::IndexingNotFinished));
    document.cancel_large_save();
    document
        .wait_for_large_save()
        .expect("settle the first save");
    assert_eq!(document.large_status(), LargeStatus::Ready);
    assert!(document.is_large_mode());
    assert_eq!(document.large_save_progress(), None);
}

#[test]
fn edits_are_refused_while_saving() {
    let fixture = Fixture::new();
    let path = fixture.csv(300);
    let mut document = open_without_header(&path);
    let before = source(&document);
    let copy = fixture.copy_path();
    let _job = document
        .start_large_save(Some(&copy))
        .expect("start the save");

    assert!(!document.can_edit_cells());
    assert_eq!(
        document.set_large_cell("Y", 2, 1),
        Err(CsvError::RowNotEditable)
    );
    assert_eq!(
        document.notice(),
        Some(DocumentNotice::EditRejected(CsvError::RowNotEditable))
    );

    document.cancel_large_save();
    document.wait_for_large_save().expect("settle the save");
    let after = source(&document);
    assert!(Arc::ptr_eq(&before, &after));
    assert!(after.overlay.is_empty());
}

#[test]
fn cancel_before_the_writer_starts_restores_the_original() {
    let fixture = Fixture::new();
    let path = fixture.large_csv(2_000_000);
    let mut document = open_without_header(&path);
    let before = source(&document);
    document.set_large_cell("X", 2, 1).expect("edit the cell");
    let copy = fixture.copy_path();

    let _job = document
        .start_large_save(Some(&copy))
        .expect("start the save as");
    document.cancel_large_save();
    document
        .wait_for_large_save()
        .expect("a cancelled save is not an error");

    assert!(!copy.exists(), "a cancelled save leaves no target file");
    let after = source(&document);
    assert!(Arc::ptr_eq(&before, &after));
    assert_eq!(after.cell(2, 1), "X");
    assert_eq!(after.overlay.count(), 1);
    assert_eq!(document.undo_action(), Some(UndoAction::EditCell));
    assert!(document.can_undo());
    assert_eq!(document.large_status(), LargeStatus::Ready);
    assert_eq!(document.file_path(), Some(path.as_path()));
}

#[test]
fn cancel_mid_copy_restores_the_original() {
    let fixture = Fixture::new();
    let path = fixture.large_csv(2_000_000);
    let mut document = open_without_header(&path);
    let before_inode = inode(&path);
    let before = source(&document);
    document.set_large_cell("X", 2, 1).expect("edit the cell");
    let _job = document
        .start_large_save(None)
        .expect("start the in-place save");

    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        match document.large_save_progress() {
            Some(progress) if progress > 0.0 && progress < 1.0 => break,
            Some(1.0) => panic!("the save finished before it could be cancelled"),
            Some(_) => {}
            None => panic!("the save stopped reporting progress"),
        }
        assert!(Instant::now() < deadline, "the large save timed out");
        std::thread::yield_now();
    }
    document.cancel_large_save();
    document
        .wait_for_large_save()
        .expect("a cancelled save is not an error");

    assert_eq!(inode(&path), before_inode, "the original file is untouched");
    let after = source(&document);
    assert!(Arc::ptr_eq(&before, &after));
    assert_eq!(after.cell(2, 1), "X");
    assert_eq!(after.overlay.count(), 1);
    assert!(document.can_undo());
    assert_eq!(document.large_status(), LargeStatus::Ready);
    assert_eq!(document.file_path(), Some(path.as_path()));
}

#[test]
fn a_failed_save_restores_the_edits_and_undo() {
    let fixture = Fixture::new();
    let path = fixture.csv(300);
    let mut document = open_without_header(&path);
    let before = source(&document);
    document.set_large_cell("X", 2, 1).expect("edit the cell");
    // A missing parent makes the streaming writer fail before it installs.
    let destination = fixture.directory().join("missing").join("copy.csv");

    document
        .start_large_save(Some(&destination))
        .expect("start the failing save");
    let error = document
        .wait_for_large_save()
        .expect_err("the missing parent must fail the save");

    assert!(matches!(error, LargeFileSaveError::Io(_)));
    assert!(!destination.exists());
    let after = source(&document);
    assert!(Arc::ptr_eq(&before, &after));
    assert_eq!(after.cell(2, 1), "X");
    assert_eq!(after.overlay.count(), 1);
    assert_eq!(document.undo_action(), Some(UndoAction::EditCell));
    assert!(document.can_undo());
    assert_eq!(document.large_status(), LargeStatus::Ready);
    assert_eq!(document.file_path(), Some(path.as_path()));
    assert!(document.is_dirty());

    assert!(document.undo());
    assert_eq!(source(&document).cell(2, 1), "v2");
    assert!(document.can_redo());
}

#[test]
fn job_wait_reports_the_adopted_source_before_the_document_applies_it() {
    let fixture = Fixture::new();
    let path = fixture.csv(300);
    let mut document = open_without_header(&path);
    let before = source(&document);
    document.set_large_cell("X", 2, 1).expect("edit the cell");
    let copy = fixture.copy_path();

    let job = document
        .start_large_save(Some(&copy))
        .expect("start the save as");
    let outcome = job.wait();

    let LargeSaveJobOutcome::Saved(adoption) = outcome else {
        panic!("expected the adopted source, got {outcome:?}");
    };
    assert!(!Arc::ptr_eq(&before, &adoption.source));
    assert_eq!(adoption.source.cell(2, 1), "X");
    assert!(adoption.indexer.is_finished());

    // The document still holds the save until it applies the outcome.
    assert!(matches!(document.large_status(), LargeStatus::Saving(_)));
    document
        .wait_for_large_save()
        .expect("apply the cached outcome");
    assert!(!Arc::ptr_eq(&before, &source(&document)));
    assert_eq!(document.file_path(), Some(copy.as_path()));
    assert_eq!(document.large_status(), LargeStatus::Ready);
    assert!(!document.is_dirty());
}

#[test]
fn the_error_reaches_job_wait_and_the_document() {
    let fixture = Fixture::new();
    let path = fixture.csv(300);
    let mut document = open_without_header(&path);
    document.set_large_cell("X", 2, 1).expect("edit the cell");
    let destination = fixture.directory().join("missing").join("copy.csv");

    let job = document
        .start_large_save(Some(&destination))
        .expect("start the failing save");
    let LargeSaveJobOutcome::Failed(first) = job.wait() else {
        panic!("expected a typed failure");
    };
    assert!(matches!(
        first,
        LargeFileSaveError::Io(ref error) if error.kind() == std::io::ErrorKind::NotFound
    ));

    let second = document
        .wait_for_large_save()
        .expect_err("the document still reports the typed failure");
    assert!(matches!(
        second,
        LargeFileSaveError::Io(ref error) if error.kind() == std::io::ErrorKind::NotFound
    ));
    assert_eq!(source(&document).cell(2, 1), "X");
    assert_eq!(document.large_status(), LargeStatus::Ready);
    assert!(document.can_undo());
}
