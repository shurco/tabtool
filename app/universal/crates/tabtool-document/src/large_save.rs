//! Background streaming save for large documents.
//!
//! [`Document::start_large_save`] takes the mapped state out of the document,
//! captures a [`tabtool_large::LargeSaveSnapshot`] and runs the two-phase saver
//! on one worker thread. [`LargeSaveJob`] shares its progress and cancellation
//! flag with the document, and [`LargeSaveJob::wait`] joins the worker and
//! reports the adopted source or the typed failure.
//!
//! The worker prepares the temporary file while the original mapping is still
//! alive, then drops the mapping and installs the file. A successful save
//! adopts a fresh mapping, a full index and a source with the same overlay and
//! column ids. A cancelled or failed save restores the original mapping from a
//! seeded index, so no rescan is needed and the undo history stays usable.
//!
//! [`Document::start_large_save`]: crate::Document::start_large_save

use std::fmt;
use std::io;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::JoinHandle;

use tabtool_large::{LargeCSVSource, LargeFileIndexer, LargeFileSaveError, MappedFile, lock};

use crate::large_save_worker::{SaveInputs, WorkerOutcome};
use crate::large_state::LargeState;

mod lifecycle;

/// How a large save ended.
///
/// The job caches this value, so a caller may wait on the job before the
/// document applies the outcome; the document still receives the adopted
/// source or the same typed failure.
#[derive(Clone, Debug)]
pub enum LargeSaveJobOutcome {
    /// The file was written and is ready to replace the source on screen.
    Saved(LargeSaveAdoption),
    /// The document already adopted this save, so the job no longer holds the
    /// target mapping. A later waiter gets this instead of [`Self::Saved`].
    Applied,
    /// The save was cancelled; the original state was restored.
    Cancelled,
    /// The save failed; the original state was restored when possible.
    Failed(LargeFileSaveError),
}

/// The source and mapping of a written large file.
#[derive(Clone)]
pub struct LargeSaveAdoption {
    /// The mapping of the written file.
    pub file: Arc<MappedFile>,
    /// The completed index of the written file.
    pub indexer: Arc<LargeFileIndexer>,
    /// The rows of the written file with the kept overlay and column ids.
    pub source: Arc<LargeCSVSource>,
}

impl fmt::Debug for LargeSaveAdoption {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LargeSaveAdoption")
            .field("file", &self.file)
            .finish_non_exhaustive()
    }
}

/// A running large save.
///
/// The document keeps its own handle, so [`Document::cancel_large_save`],
/// [`Document::large_save_progress`] and [`Document::wait_for_large_save`] work
/// even when the caller drops this value.
///
/// [`Document::cancel_large_save`]: crate::Document::cancel_large_save
/// [`Document::large_save_progress`]: crate::Document::large_save_progress
/// [`Document::wait_for_large_save`]: crate::Document::wait_for_large_save
pub struct LargeSaveJob {
    pub(crate) shared: Arc<Shared>,
}

impl LargeSaveJob {
    /// The share of the file written, from 0 to 1.
    pub fn progress(&self) -> f64 {
        f64::from_bits(self.shared.progress.load(Ordering::Acquire))
    }

    /// Asks the writer to stop.
    ///
    /// A save that already replaced the file still finishes as
    /// [`LargeSaveJobOutcome::Saved`], matching the Swift `cancelLargeSave`.
    pub fn cancel(&self) {
        self.shared.cancel.store(true, Ordering::Release);
    }

    /// Whether the worker has finished, without joining it.
    ///
    /// A caller that polls progress needs this to notice an early failure: the
    /// progress value can stay below 1.0 when the writer cannot start.
    #[must_use]
    pub fn is_finished(&self) -> bool {
        let inner = lock(&self.shared.inner);
        inner
            .handle
            .as_ref()
            .is_some_and(std::thread::JoinHandle::is_finished)
    }

    /// Waits for the worker and reports how it ended.
    ///
    /// Every handle receives the same cached outcome, including the original
    /// typed [`LargeFileSaveError`]. The document applies the outcome only in
    /// [`Document::wait_for_large_save`]. After that call installs the adopted
    /// source, it replaces the cached value with [`LargeSaveJobOutcome::Applied`]
    /// so a job handle kept by the caller cannot keep the target mapped and
    /// block the next in-place save on Windows. A [`LargeSaveJobOutcome::Saved`]
    /// value returned to the caller still owns the target mapping: drop it
    /// before another in-place save.
    ///
    /// [`Document::wait_for_large_save`]: crate::Document::wait_for_large_save
    pub fn wait(self) -> LargeSaveJobOutcome {
        wait_shared(&self.shared)
    }
}

impl fmt::Debug for LargeSaveJob {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LargeSaveJob")
            .field("destination", &self.shared.destination)
            .field("progress", &self.progress())
            .field("cancelled", &self.shared.cancel.load(Ordering::Acquire))
            .finish()
    }
}

/// State shared by every handle to one save.
pub(crate) struct Shared {
    progress: AtomicU64,
    cancel: AtomicBool,
    /// The target the worker writes, also the path adoption binds.
    destination: PathBuf,
    /// The document revision the snapshot describes.
    revision: u64,
    inner: Mutex<Inner>,
    /// Signals a waiter that another handle cached the outcome.
    finished: Condvar,
}

/// The worker handle, the pending inputs, the restore state and the cached
/// outcome.
struct Inner {
    handle: Option<JoinHandle<WorkerOutcome>>,
    /// The inputs wait here only until the worker takes them; a failed spawn
    /// restores them without starting a thread.
    pending: Option<SaveInputs>,
    restored: Option<LargeState>,
    cached: Option<LargeSaveJobOutcome>,
}

/// Joins the worker once and caches the outcome for any other handle.
fn wait_shared(shared: &Arc<Shared>) -> LargeSaveJobOutcome {
    let handle = { lock(&shared.inner).handle.take() };
    let Some(handle) = handle else {
        return cached_outcome(shared);
    };
    let worker = handle.join().unwrap_or_else(|_| WorkerOutcome {
        outcome: LargeSaveJobOutcome::Failed(LargeFileSaveError::Io(io::Error::other(
            "the large save thread panicked",
        ))),
        restored: None,
    });
    {
        let mut inner = lock(&shared.inner);
        inner.restored = worker.restored;
        inner.cached = Some(worker.outcome.clone());
        shared.finished.notify_all();
    }
    worker.outcome
}

/// The outcome of a save another handle already joined.
fn cached_outcome(shared: &Arc<Shared>) -> LargeSaveJobOutcome {
    let inner = lock(&shared.inner);
    let inner = shared
        .finished
        .wait_while(inner, |inner| inner.cached.is_none())
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    inner
        .cached
        .clone()
        .expect("the waiter was notified with a cached outcome")
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;
    use std::sync::mpsc;
    use std::time::Duration;

    use tabtool_core::CsvConfiguration;
    use tabtool_large::{CellOverlay, LargeCSVSource, LargeFileIndexer, MappedFile, PreparedSave};

    use super::*;
    use crate::Document;
    use crate::large_save_worker::execute_save;
    use crate::types::{DocumentPhase, LargeStatus};

    /// The Windows constraint, checked without a Windows build: no reference
    /// to the original mapping may survive until the install.
    #[test]
    fn the_original_mapping_is_released_before_install() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let path = directory.path().join("big.csv");
        let text = (0..300)
            .map(|row| format!("r{row},v{},\"q {row}\"", row % 7))
            .collect::<Vec<_>>()
            .join("\n");
        std::fs::write(&path, text).expect("fixture file is written");

        let mut document = Document::new(CsvConfiguration::default());
        document
            .open_large(&path, None, false, 0)
            .expect("open the fixture");
        document.large_state().expect("large state").indexer.wait();
        document.poll_large_indexing();
        if document.configuration().first_row_as_header {
            document.set_large_header_enabled(false);
        }
        document.set_large_cell("X", 2, 1).expect("edit the cell");
        let weak = Arc::downgrade(&document.large_state().expect("large state").file);
        let copy = directory.path().join("copy.csv");

        let inputs = document
            .prepare_save(Some(&copy))
            .expect("the synchronous half starts");
        assert!(
            weak.upgrade().is_some(),
            "the mapping is alive while preparing"
        );

        let outcome = execute_save(
            inputs,
            &AtomicBool::new(false),
            |_| {},
            || {
                assert!(
                    weak.upgrade().is_none(),
                    "the original mapping is released before install"
                );
            },
            PreparedSave::install,
        );

        assert!(matches!(outcome.outcome, LargeSaveJobOutcome::Saved(_)));
        assert!(weak.upgrade().is_none());
        let saved = std::fs::read_to_string(&copy).expect("the copy is readable");
        assert_eq!(saved.lines().nth(2), Some("\"r2\",\"X\",\"q 2\""));
    }

    /// `is_finished` is published before the indexer thread exits. The save
    /// must join that thread before the mapping release, or the thread can keep
    /// an `Arc<MappedFile>` alive until the Windows install fails.
    #[test]
    fn the_indexer_thread_is_joined_before_the_mapping_is_released() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let path = directory.path().join("big.csv");
        let text = (0..300)
            .map(|row| format!("r{row},v{},\"q {row}\"", row % 7))
            .collect::<Vec<_>>()
            .join("\n");
        std::fs::write(&path, text).expect("fixture file is written");
        let configuration = CsvConfiguration::default();

        let file = Arc::new(MappedFile::open(&path).expect("fixture opens"));
        let indexer = Arc::new(LargeFileIndexer::new(
            Arc::clone(&file),
            configuration.clone(),
            4_096,
            Duration::ZERO,
        ));
        let (entered_sender, entered) = mpsc::channel();
        let (release, released) = mpsc::channel::<()>();
        indexer.start(move |progress| {
            if progress.is_finished {
                entered_sender
                    .send(())
                    .expect("the test waits for the final callback");
                released.recv().expect("the test releases the callback");
            }
        });
        entered
            .recv_timeout(Duration::from_secs(10))
            .expect("the final callback runs");
        assert!(
            indexer.is_finished(),
            "the finished flag is set before the callback returns"
        );

        let source = Arc::new(LargeCSVSource::new(
            Arc::clone(&file),
            configuration.clone(),
            Arc::clone(&indexer),
            CellOverlay::new(),
            Vec::new(),
        ));
        let mut document = Document::new(configuration.clone());
        document.table = None;
        document.large = Some(LargeState {
            source,
            indexer: Arc::clone(&indexer),
            file: Arc::clone(&file),
        });
        document.file_path = Some(path.clone());
        document.phase = DocumentPhase::Ready;
        document.bump_revision();
        document.saved_revision = document.revision;
        document.large_status = LargeStatus::Ready;
        document.set_large_cell("X", 2, 1).expect("edit the cell");
        let weak = Arc::downgrade(&file);
        drop(indexer);
        drop(file);
        let copy = directory.path().join("copy.csv");
        let (inputs_sender, inputs) = mpsc::channel();

        std::thread::scope(|scope| {
            scope.spawn(move || {
                let prepared = document
                    .prepare_save(Some(&copy))
                    .expect("the synchronous half starts");
                inputs_sender
                    .send(prepared)
                    .expect("the test receives the inputs");
            });
            std::thread::sleep(Duration::from_millis(100));
            assert!(
                matches!(inputs.try_recv(), Err(mpsc::TryRecvError::Empty)),
                "prepare_save waits for the indexer thread"
            );
            release.send(()).expect("release the indexer callback");
            let inputs = inputs
                .recv_timeout(Duration::from_secs(10))
                .expect("prepare_save finishes");
            let outcome = execute_save(
                inputs,
                &AtomicBool::new(false),
                |_| {},
                || {
                    assert!(
                        weak.upgrade().is_none(),
                        "the indexer thread ended and the mapping is released"
                    );
                },
                PreparedSave::install,
            );
            assert!(matches!(outcome.outcome, LargeSaveJobOutcome::Saved(_)));
        });
    }

    /// A directory sync after the swap may fail; the target already holds the
    /// new bytes, so the worker must adopt it instead of restoring the old
    /// seeded index.
    #[test]
    fn a_post_swap_sync_failure_is_adopted() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let path = directory.path().join("big.csv");
        let text = (0..300)
            .map(|row| format!("r{row},v{},\"q {row}\"", row % 7))
            .collect::<Vec<_>>()
            .join("\n");
        std::fs::write(&path, text).expect("fixture file is written");

        let mut document = Document::new(CsvConfiguration::default());
        document
            .open_large(&path, None, false, 0)
            .expect("open the fixture");
        document.large_state().expect("large state").indexer.wait();
        document.poll_large_indexing();
        if document.configuration().first_row_as_header {
            document.set_large_header_enabled(false);
        }
        document.set_large_cell("X", 2, 1).expect("edit the cell");
        let copy = directory.path().join("copy.csv");

        let inputs = document
            .prepare_save(Some(&copy))
            .expect("the synchronous half starts");
        let outcome = execute_save(
            inputs,
            &AtomicBool::new(false),
            |_| {},
            || {},
            |prepared| {
                prepared.install_with_directory_sync(|_| {
                    Err(io::Error::other("injected directory sync failure"))
                })
            },
        );

        let LargeSaveJobOutcome::Saved(adoption) = outcome.outcome else {
            panic!("a post-swap sync failure must keep the adopted file");
        };
        assert!(outcome.restored.is_none(), "the old index is not restored");
        assert!(adoption.indexer.is_finished());
        assert_eq!(adoption.source.cell(2, 1), "X");
        let saved = std::fs::read_to_string(&copy).expect("the copy is readable");
        assert_eq!(saved.lines().nth(2), Some("\"r2\",\"X\",\"q 2\""));
    }

    /// The worker can report `SourceChanged` without a prior poll. Restoring
    /// the mapping then must not publish `Ready` for a file that no longer
    /// matches; the stale banner has to appear immediately.
    #[test]
    fn a_restored_source_is_checked_for_staleness_immediately() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let path = directory.path().join("big.csv");
        std::fs::write(&path, b"a,b\n1,2\n3,4\n").expect("fixture file is written");
        let mut document = Document::new(CsvConfiguration::default());
        document
            .open_large(&path, None, false, 0)
            .expect("open the fixture");
        document.large_state().expect("large state").indexer.wait();
        document.poll_large_indexing();
        let state = document.large.take().expect("the mapping is taken");

        std::fs::write(&path, b"x\n").expect("the fixture is truncated");

        document.restore_large_state(state);

        assert_eq!(
            document.large_status(),
            LargeStatus::Stale { appended: false }
        );
        assert_eq!(document.phase(), &DocumentPhase::Ready);
    }
}
