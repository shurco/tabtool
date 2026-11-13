//! Port of `CSVDocumentLargeSavingTests.swift`, plus the two-phase install
//! cases: adoption, restore, Busy and the cached job error.

mod support;

use std::collections::BTreeSet;
use std::path::Path;
use std::sync::Arc;

use support::{
    Fixture, inode, lines, open_indexing, open_without_header, saving, source, start_error,
};
use tabtool_core::{CsvConfiguration, CsvError};
use tabtool_document::{DocumentError, DocumentNotice, LargeStatus, StdStorage, UndoAction};
use tabtool_large::{LargeFileSaveError, Staleness};

#[test]
fn save_writes_the_edits_and_adopts_the_written_file() {
    let fixture = Fixture::new();
    let path = fixture.csv(300);
    let mut document = open_without_header(&path);
    let original = lines(&path);
    let before = source(&document);
    let before_ids = before.column_ids();
    document.selection_mut().rows.insert(2);
    document.set_large_cell("X", 2, 1).expect("edit the cell");

    document
        .save_large_blocking(None)
        .expect("save the edits in place");

    let saved = lines(&path);
    assert_eq!(saved.len(), original.len());
    assert_eq!(saved[2], "\"r2\",\"X\",\"q 2\"");
    assert!(
        saved
            .iter()
            .enumerate()
            .all(|(row, line)| row == 2 || *line == original[row])
    );
    let after = source(&document);
    assert!(!Arc::ptr_eq(&before, &after));
    assert_eq!(after.column_ids(), before_ids);
    assert!(after.overlay.is_empty());
    assert_eq!(after.cell(2, 1), "X");
    assert_eq!(after.staleness(), Staleness::Unchanged);
    assert_eq!(document.large_status(), LargeStatus::Ready);
    assert!(!document.is_dirty());
    assert_eq!(document.file_path(), Some(path.as_path()));
    assert_eq!(document.selection().rows, BTreeSet::from([2]));
    assert_eq!(document.undo_action(), Some(UndoAction::EditCell));
    assert!(document.can_undo());
}

#[test]
fn save_without_edits_leaves_the_file_alone() {
    let fixture = Fixture::new();
    let path = fixture.csv(300);
    let mut document = open_without_header(&path);
    let bytes = std::fs::read(&path).expect("fixture is readable");
    let before_inode = inode(&path);

    document
        .save_large_blocking(None)
        .expect("a no-edit save succeeds");

    assert_eq!(std::fs::read(&path).expect("fixture is readable"), bytes);
    assert_eq!(inode(&path), before_inode);
    assert_eq!(document.large_status(), LargeStatus::Ready);
    assert!(document.is_large_mode());
    assert!(!document.is_dirty());
    assert_eq!(document.large_save_progress(), None);
}

#[test]
fn save_is_refused_until_indexing_finishes() {
    let fixture = Fixture::new();
    let path = fixture.large_csv(500_000);
    let mut document = open_indexing(&path);
    assert!(!document.can_edit_cells());

    let error = start_error(&mut document, None);

    assert!(matches!(error, LargeFileSaveError::IndexingNotFinished));
    assert!(matches!(document.large_status(), LargeStatus::Indexing(_)));
    assert!(!document.is_dirty());
}

#[test]
fn edits_are_refused_while_saving_and_cells_are_not_editable() {
    let fixture = Fixture::new();
    let path = fixture.csv(300);
    let mut document = open_without_header(&path);
    let copy = fixture.copy_path();
    let _job = document
        .start_large_save(Some(&copy))
        .expect("start the save as");

    assert!(saving(&document));
    assert!(!document.can_edit_cells());
    assert_eq!(
        document.set_large_cell("X", 2, 1),
        Err(CsvError::RowNotEditable)
    );
    assert_eq!(
        document.notice(),
        Some(DocumentNotice::EditRejected(CsvError::RowNotEditable))
    );

    document
        .wait_for_large_save()
        .expect("apply the save outcome");
    assert!(document.can_edit_cells());
}

#[test]
fn undo_after_save_restores_the_pre_edit_value() {
    let fixture = Fixture::new();
    let path = fixture.csv(300);
    let mut document = open_without_header(&path);
    document.set_large_cell("X", 2, 1).expect("edit the cell");

    document
        .save_large_blocking(None)
        .expect("save the edits in place");

    assert!(document.undo());
    assert_eq!(source(&document).cell(2, 1), "v2");
    assert!(document.is_dirty());
    assert!(document.redo());
    assert_eq!(source(&document).cell(2, 1), "X");
    assert!(!document.is_dirty());
}

#[test]
fn save_as_writes_the_new_file_and_keeps_the_original() {
    let fixture = Fixture::new();
    let path = fixture.csv(300);
    let mut document = open_without_header(&path);
    let original = std::fs::read(&path).expect("fixture is readable");
    let before = source(&document);
    let before_ids = before.column_ids();
    document.set_large_cell("X", 2, 1).expect("edit the cell");
    let copy = fixture.copy_path();

    document
        .save_large_blocking(Some(&copy))
        .expect("save a copy");

    assert_eq!(std::fs::read(&path).expect("fixture is readable"), original);
    assert_eq!(lines(&copy)[2], "\"r2\",\"X\",\"q 2\"");
    assert_eq!(document.file_path(), Some(copy.as_path()));
    let after = source(&document);
    assert!(!Arc::ptr_eq(&before, &after));
    assert_eq!(after.column_ids(), before_ids);
    assert!(after.overlay.is_empty());
    assert_eq!(after.cell(2, 1), "X");
    assert_eq!(after.staleness(), Staleness::Unchanged);
    assert_eq!(document.large_status(), LargeStatus::Ready);
    assert!(!document.is_dirty());
}

#[test]
fn save_as_over_an_existing_file_replaces_it() {
    let fixture = Fixture::new();
    let path = fixture.csv(300);
    let mut document = open_without_header(&path);
    let copy = fixture.copy_path();
    std::fs::write(&copy, b"old,file\n1,2\n").expect("the copy fixture is written");
    document.set_large_cell("X", 2, 1).expect("edit the cell");

    document
        .save_large_blocking(Some(&copy))
        .expect("save over the existing target");

    let saved = lines(&copy);
    assert_eq!(saved[2], "\"r2\",\"X\",\"q 2\"");
    assert_eq!(document.file_path(), Some(copy.as_path()));
    assert_eq!(source(&document).cell(2, 1), "X");
    assert!(!document.is_dirty());
}

#[test]
fn truncated_file_is_marked_stale() {
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
fn appended_file_is_marked_appended_and_refuses_the_save() {
    let fixture = Fixture::new();
    let path = fixture.csv(300);
    let mut document = open_without_header(&path);
    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .open(&path)
        .expect("fixture opens for appending");
    use std::io::Write;
    writeln!(file, "r300,v6,\"q 300\"").expect("append a row");
    document.refresh_large_staleness();
    assert_eq!(
        document.large_status(),
        LargeStatus::Stale { appended: true }
    );

    let error = start_error(&mut document, Some(&fixture.copy_path()));

    assert!(matches!(error, LargeFileSaveError::SourceChanged));
    assert_eq!(
        document.large_status(),
        LargeStatus::Stale { appended: true }
    );
    assert!(document.is_large_mode());
}

#[test]
fn install_failure_restores_the_original_source() {
    let fixture = Fixture::new();
    let path = fixture.csv(300);
    let mut document = open_without_header(&path);
    let original = lines(&path);
    let before_ids = source(&document).column_ids();
    document.selection_mut().rows.insert(2);
    document.set_large_cell("X", 2, 1).expect("edit the cell");
    // A directory at the target makes the swap fail after a successful
    // prepare, which is the only install failure the platform can force.
    let target = fixture.directory().join("copy.csv");
    std::fs::create_dir(&target).expect("the conflicting target directory is created");

    let error = document
        .save_large_blocking(Some(&target))
        .expect_err("replacing a directory fails");

    assert!(matches!(error, LargeFileSaveError::Io(_)));
    assert_eq!(lines(&path), original);
    assert!(target.is_dir());
    assert_eq!(document.file_path(), Some(path.as_path()));
    assert_eq!(document.large_status(), LargeStatus::Ready);
    assert!(document.is_large_mode());
    let restored = document.large_state().expect("the original is restored");
    assert!(
        restored.indexer.is_finished(),
        "the index is seeded, not rescanned"
    );
    assert_eq!(restored.source.column_ids(), before_ids);
    assert_eq!(restored.source.cell(2, 1), "X");
    assert_eq!(restored.source.overlay.count(), 1);
    assert_eq!(restored.source.staleness(), Staleness::Unchanged);
    assert_eq!(document.selection().rows, BTreeSet::from([2]));
    assert_eq!(document.undo_action(), Some(UndoAction::EditCell));
    assert!(document.can_undo());
    assert!(document.is_dirty());
}

#[test]
fn a_document_replacing_operation_during_save_is_busy() {
    let fixture = Fixture::new();
    let path = fixture.csv(300);
    let mut document = open_without_header(&path);
    document.set_large_cell("X", 2, 1).expect("edit the cell");
    let copy = fixture.copy_path();
    let _job = document
        .start_large_save(Some(&copy))
        .expect("start the save as");
    let configuration = CsvConfiguration::default();
    let other = fixture.directory().join("other.csv");

    assert_eq!(
        document.load_from_bytes(b"x", None, Some(configuration.clone()), false),
        Err(DocumentError::Busy)
    );
    assert_eq!(
        document.load_from_path(&other, &StdStorage, None, false),
        Err(DocumentError::Busy)
    );
    assert_eq!(
        document.new_document(configuration.clone()),
        Err(DocumentError::Busy)
    );
    assert_eq!(
        document.open_large(&other, None, false, 0),
        Err(DocumentError::Busy)
    );
    assert_eq!(
        document.open_file(&other, &StdStorage, None, false, 0),
        Err(DocumentError::Busy)
    );
    assert_eq!(document.reload_large(&StdStorage), Err(DocumentError::Busy));
    assert_eq!(
        document.reopen_large(configuration.clone(), true),
        Err(DocumentError::Busy)
    );
    assert_eq!(
        document.apply_parsing_configuration(configuration, true, &StdStorage),
        Err(DocumentError::Busy)
    );
    assert_eq!(
        document.revert_to_saved(&StdStorage),
        Err(DocumentError::Busy)
    );
    assert_eq!(document.save(&StdStorage), Err(DocumentError::Busy));
    assert_eq!(
        document.save_as(Path::new("other.csv"), &StdStorage),
        Err(DocumentError::Busy)
    );

    document
        .wait_for_large_save()
        .expect("apply the save outcome");
    assert!(document.is_large_mode());
    assert_eq!(document.file_path(), Some(copy.as_path()));
}

#[test]
fn can_edit_cells_is_false_while_saving() {
    let fixture = Fixture::new();
    let path = fixture.csv(300);
    let mut document = open_without_header(&path);
    assert!(document.can_edit_cells());
    let copy = fixture.copy_path();
    let _job = document
        .start_large_save(Some(&copy))
        .expect("start the save as");
    assert!(saving(&document));
    assert!(!document.can_edit_cells());
    document
        .wait_for_large_save()
        .expect("apply the save outcome");
    assert!(document.can_edit_cells());
}
