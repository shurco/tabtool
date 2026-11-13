//! Port of `CSVDocumentLargeSavingTests.swift` for TabTool's Rust document.
//!
//! Task 9a covers the save job itself: the index and save gates, progress,
//! cancellation, failure and the restore of the original mapping. Adoption of
//! the written file is Task 9b, so a successful in-place save leaves the
//! document without large content until then.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use tabtool_core::{CsvConfiguration, CsvError};
use tabtool_document::{Document, DocumentNotice, LargeSaveJobOutcome, LargeStatus, UndoAction};
use tabtool_large::LargeFileSaveError;

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

    fn copy_path(&self) -> PathBuf {
        self.directory.path().join("copy.csv")
    }

    /// Rows like `r5,v5,"q 5"`; the second field repeats every seven rows.
    fn csv(&self, rows: usize) -> PathBuf {
        let path = self.path();
        let text = (0..rows)
            .map(|row| format!("r{row},v{},\"q {row}\"", row % 7))
            .collect::<Vec<_>>()
            .join("\n");
        std::fs::write(&path, text).expect("fixture file is written");
        path
    }

    /// A large fixture written straight to disk, so multi-megabyte files do
    /// not need to be formatted into one `String` first.
    fn large_csv(&self, rows: usize) -> PathBuf {
        let path = self.path();
        let file = std::fs::File::create(&path).expect("fixture file is created");
        let mut writer = std::io::BufWriter::new(file);
        for row in 0..rows {
            writeln!(writer, "r{row},v{},\"q {row}\"", row % 7).expect("fixture row is written");
        }
        writer.flush().expect("fixture is flushed");
        path
    }
}

fn open_ready(path: &Path) -> Document {
    let mut document = Document::new(CsvConfiguration::default());
    document
        .open_large(path, None, false, 0)
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
    let mut document = open_ready(path);
    if document.configuration().first_row_as_header {
        document.set_large_header_enabled(false);
    }
    document
}

/// A document whose indexer was cancelled after its first step, so the index
/// gate can be observed deterministically.
fn open_indexing(path: &Path) -> Document {
    let mut document = Document::new(CsvConfiguration::default());
    document
        .open_large(path, None, false, 0)
        .expect("open the large fixture");
    let state = document.large_state().expect("large state");
    assert!(
        !state.indexer.is_finished(),
        "the indexer finished before it could be paused"
    );
    state.indexer.cancel();
    state.indexer.wait();
    document.poll_large_indexing();
    document
}

fn source(document: &Document) -> Arc<tabtool_large::LargeCSVSource> {
    Arc::clone(&document.large_state().expect("large state").source)
}

fn lines(path: &Path) -> Vec<String> {
    std::fs::read_to_string(path)
        .expect("fixture is readable")
        .lines()
        .map(str::to_owned)
        .collect()
}

#[cfg(unix)]
fn inode(path: &Path) -> u64 {
    use std::os::unix::fs::MetadataExt;
    std::fs::metadata(path).expect("fixture metadata").ino()
}

#[cfg(not(unix))]
fn inode(_path: &Path) -> u64 {
    0
}

fn start_error(document: &mut Document, destination: Option<&Path>) -> LargeFileSaveError {
    match document.start_large_save(destination) {
        Ok(_) => panic!("the large save was expected to be refused"),
        Err(error) => error,
    }
}

/// Waits until the worker reports a perfect fraction; the document keeps the
/// job until `wait_for_large_save`, so the status stays `Saving`.
fn wait_until_complete(document: &Document) {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        match document.large_save_progress() {
            Some(progress) if progress >= 1.0 => return,
            Some(_) => {}
            None => panic!("the save stopped reporting progress"),
        }
        assert!(Instant::now() < deadline, "the large save timed out");
        std::thread::yield_now();
    }
}

#[test]
fn save_without_edits_leaves_the_file_alone() {
    let fixture = Fixture::new();
    let path = fixture.csv(300);
    let mut document = open_without_header(&path);
    let bytes = std::fs::read(&path).expect("fixture is readable");
    let before_inode = inode(&path);
    let before = source(&document);

    document
        .start_large_save(None)
        .expect("start the no-edit save");
    document
        .wait_for_large_save()
        .expect("a no-edit save succeeds");

    assert_eq!(std::fs::read(&path).expect("fixture is readable"), bytes);
    assert_eq!(inode(&path), before_inode);
    assert!(Arc::ptr_eq(&before, &source(&document)));
    assert_eq!(document.large_status(), LargeStatus::Ready);
    assert!(document.is_large_mode());
    assert_eq!(document.large_save_progress(), None);
}

#[test]
fn save_writes_the_edits_and_leaves_adoption_to_the_next_task() {
    let fixture = Fixture::new();
    let path = fixture.csv(300);
    let mut document = open_without_header(&path);
    let original = lines(&path);
    document.set_large_cell("X", 2, 1).expect("edit the cell");

    let job = document
        .start_large_save(None)
        .expect("start the in-place save");
    assert!(matches!(document.large_status(), LargeStatus::Saving(_)));
    match job.wait() {
        LargeSaveJobOutcome::Saved { destination } => assert_eq!(destination, path),
        other => panic!("expected the written destination, got {other:?}"),
    }
    document
        .wait_for_large_save()
        .expect("apply the save outcome");

    let saved = lines(&path);
    assert_eq!(saved.len(), original.len());
    assert_eq!(saved[2], "\"r2\",\"X\",\"q 2\"");
    assert!(
        saved
            .iter()
            .enumerate()
            .all(|(row, line)| row == 2 || *line == original[row])
    );
    // Task 9b adopts the written file; 9a only reports the save.
    assert!(!document.is_large_mode());
    assert_eq!(document.large_status(), LargeStatus::None);
    assert_eq!(document.file_path(), Some(path.as_path()));
    assert_eq!(document.large_save_progress(), None);
}

#[test]
fn save_as_writes_the_new_file_and_keeps_the_original_state() {
    let fixture = Fixture::new();
    let path = fixture.csv(300);
    let mut document = open_without_header(&path);
    let original = std::fs::read(&path).expect("fixture is readable");
    let before = source(&document);
    document.set_large_cell("X", 2, 1).expect("edit the cell");
    let copy = fixture.copy_path();

    let job = document
        .start_large_save(Some(&copy))
        .expect("start the save as");
    match job.wait() {
        LargeSaveJobOutcome::Saved { destination } => assert_eq!(destination, copy),
        other => panic!("expected the written destination, got {other:?}"),
    }
    document
        .wait_for_large_save()
        .expect("apply the save outcome");

    assert_eq!(std::fs::read(&path).expect("fixture is readable"), original);
    assert_eq!(lines(&copy)[2], "\"r2\",\"X\",\"q 2\"");
    assert_eq!(document.file_path(), Some(path.as_path()));
    assert!(document.is_large_mode());
    assert_eq!(document.large_status(), LargeStatus::Ready);
    let after = source(&document);
    assert!(Arc::ptr_eq(&before, &after));
    assert_eq!(after.cell(2, 1), "X");
    assert_eq!(after.overlay.count(), 1);
    assert!(document.is_dirty());
    assert_eq!(document.large_save_progress(), None);
}

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
    // A missing parent makes the streaming writer fail before it writes.
    let destination = fixture.directory.path().join("missing").join("copy.csv");

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
