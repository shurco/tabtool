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

use std::fmt;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::thread::{self, JoinHandle};

use tabtool_large::{
    LargeCSVSource, LargeFileIndexer, LargeFileSaveError, MappedFile, PreparedSave,
};

use crate::Document;
use crate::large_save_worker::{SaveInputs, WorkerOutcome, execute_save};
use crate::large_state::LargeState;
use crate::types::{DocumentPhase, LargeStatus};

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

impl Document {
    /// Starts writing a large document with its edits.
    ///
    /// `destination` is Save As; `None` writes the current file. The document
    /// must be large, ready and fully indexed. The mapped state leaves the
    /// document while the worker runs, so cells cannot be edited and
    /// [`Document::large_status`] reports [`LargeStatus::Saving`]. A save
    /// without a destination and without overlay edits only rebuilds the
    /// source, without writing.
    ///
    /// A second start while a save runs is refused, like a stale file
    /// ([`LargeFileSaveError::SourceChanged`]) or a document that is not ready
    /// ([`LargeFileSaveError::IndexingNotFinished`]).
    pub fn start_large_save(
        &mut self,
        destination: Option<&Path>,
    ) -> Result<LargeSaveJob, LargeFileSaveError> {
        let inputs = self.prepare_save(destination)?;
        let target = inputs.target.clone();
        let revision = self.revision;
        let shared = Arc::new(Shared {
            progress: AtomicU64::new(0.0f64.to_bits()),
            cancel: AtomicBool::new(false),
            destination: target,
            revision,
            inner: Mutex::new(Inner {
                handle: None,
                pending: Some(inputs),
                restored: None,
                cached: None,
            }),
            finished: Condvar::new(),
        });

        let worker_shared = Arc::clone(&shared);
        let spawned = thread::Builder::new()
            .name("tabtool-large-save".to_owned())
            .spawn(move || {
                let inputs = lock(&worker_shared.inner).pending.take();
                let Some(inputs) = inputs else {
                    return WorkerOutcome {
                        outcome: LargeSaveJobOutcome::Failed(LargeFileSaveError::Io(
                            io::Error::other("the large save inputs went missing"),
                        )),
                        restored: None,
                    };
                };
                execute_save(
                    inputs,
                    &worker_shared.cancel,
                    |fraction| {
                        worker_shared
                            .progress
                            .store(fraction.to_bits(), Ordering::Release);
                    },
                    || {},
                    PreparedSave::install,
                )
            });

        match spawned {
            Ok(handle) => {
                lock(&shared.inner).handle = Some(handle);
                self.large_save = Some(LargeSaveJob {
                    shared: Arc::clone(&shared),
                });
                Ok(LargeSaveJob { shared })
            }
            Err(error) => {
                let inputs = lock(&shared.inner).pending.take();
                match inputs {
                    Some(inputs) => self.restore_large_state(inputs.original),
                    None => self.clear_large_content(),
                }
                Err(LargeFileSaveError::Io(error))
            }
        }
    }

    /// Asks the save in flight, if any, to stop.
    ///
    /// [`Document::wait_for_large_save`] applies the result and restores the
    /// original state afterwards.
    pub fn cancel_large_save(&mut self) {
        if let Some(job) = &self.large_save {
            job.cancel();
        }
    }

    /// The share of the file written, only while a save is running.
    pub fn large_save_progress(&self) -> Option<f64> {
        match self.large_status {
            LargeStatus::Saving(_) => self.large_save.as_ref().map(LargeSaveJob::progress),
            LargeStatus::None
            | LargeStatus::Indexing(_)
            | LargeStatus::Ready
            | LargeStatus::Stale { .. } => None,
        }
    }

    /// Saves a large document and applies the outcome before returning.
    ///
    /// This is [`Document::start_large_save`] followed by
    /// [`Document::wait_for_large_save`]; the UI uses the job form so it can
    /// render progress.
    pub fn save_large_blocking(
        &mut self,
        destination: Option<&Path>,
    ) -> Result<(), LargeFileSaveError> {
        self.start_large_save(destination)?;
        self.wait_for_large_save()
    }

    /// Waits for the save in flight and applies its outcome.
    ///
    /// A successful save adopts the written file, updates the path for Save As,
    /// marks the snapshot revision saved and keeps the undo history. Cancelled
    /// and failed saves restore the original mapping, overlay and undo history;
    /// a failure returns its typed error. A second call after the job settled
    /// does nothing.
    ///
    /// After adoption the job cache holds [`LargeSaveJobOutcome::Applied`]
    /// instead of the source, so a [`LargeSaveJob`] handle kept by the caller
    /// cannot keep the target mapping alive for the next in-place save.
    pub fn wait_for_large_save(&mut self) -> Result<(), LargeFileSaveError> {
        let Some(job) = self.large_save.take() else {
            return Ok(());
        };
        let shared = Arc::clone(&job.shared);
        let outcome = job.wait();
        let restored = lock(&shared.inner).restored.take();
        match outcome {
            LargeSaveJobOutcome::Saved(adoption) => {
                self.adopt_large(adoption, shared.destination.clone());
                self.saved_revision = shared.revision;
                lock(&shared.inner).cached = Some(LargeSaveJobOutcome::Applied);
                Ok(())
            }
            LargeSaveJobOutcome::Applied => Ok(()),
            LargeSaveJobOutcome::Cancelled => {
                self.restore_or_clear(restored);
                Ok(())
            }
            LargeSaveJobOutcome::Failed(error) => {
                self.restore_or_clear(restored);
                Err(error)
            }
        }
    }

    /// Takes the saved state out of the document and captures what the worker
    /// needs.
    ///
    /// This is the synchronous first half of [`Document::start_large_save`]:
    /// it applies every gate, then leaves the document in
    /// [`LargeStatus::Saving`] without a source. A `Saving` document refuses
    /// edits and document-replacing operations until the outcome is applied.
    pub(crate) fn prepare_save(
        &mut self,
        destination: Option<&Path>,
    ) -> Result<SaveInputs, LargeFileSaveError> {
        if self.rereading || self.large_save.is_some() {
            return Err(LargeFileSaveError::IndexingNotFinished);
        }
        match self.large_status {
            LargeStatus::Ready => {}
            LargeStatus::Stale { .. } => return Err(LargeFileSaveError::SourceChanged),
            LargeStatus::None | LargeStatus::Indexing(_) | LargeStatus::Saving(_) => {
                return Err(LargeFileSaveError::IndexingNotFinished);
            }
        }
        let Some(state) = self.large.as_ref() else {
            return Err(LargeFileSaveError::IndexingNotFinished);
        };
        if !state.indexer.is_finished() {
            return Err(LargeFileSaveError::IndexingNotFinished);
        }
        // `is_finished` is published before the worker's final callback and
        // thread exit. Join it here so no indexer thread can keep an
        // `Arc<MappedFile>` alive past the mapping release that the install
        // needs on Windows.
        state.indexer.wait();
        let source_path = self
            .file_path
            .clone()
            .ok_or(LargeFileSaveError::IndexingNotFinished)?;
        let target = destination
            .map(Path::to_path_buf)
            .unwrap_or_else(|| source_path.clone());
        let snapshot = state.source.save_snapshot();
        let should_write = destination.is_some() || !snapshot.edits.is_empty();
        let configuration = self.configuration.clone();
        let column_ids = state.source.column_ids();
        let overlay = state.source.overlay.copy();
        let saved_edits = snapshot.edits.clone();

        let state = self.large.take().expect("the state was checked above");
        self.table = None;
        self.large_status = LargeStatus::Saving(0.0);
        Ok(SaveInputs {
            original: state,
            snapshot,
            target,
            source_path,
            configuration,
            column_ids,
            overlay,
            saved_edits,
            should_write,
        })
    }

    /// Puts the written file on screen with the same columns and overlay.
    fn adopt_large(&mut self, adoption: LargeSaveAdoption, path: PathBuf) {
        debug_assert!(
            adoption.indexer.is_finished(),
            "the adopted index must be complete"
        );
        self.table = None;
        self.large = Some(LargeState {
            source: adoption.source,
            indexer: adoption.indexer,
            file: adoption.file,
        });
        self.file_path = Some(path);
        self.large_status = LargeStatus::Ready;
        self.phase = DocumentPhase::Ready;
    }

    /// Puts a restored mapping back and reports it as ready.
    fn restore_large_state(&mut self, state: LargeState) {
        self.table = None;
        self.large = Some(state);
        self.large_status = LargeStatus::Ready;
        self.phase = DocumentPhase::Ready;
    }

    /// Restores the original state, or clears the mode when it is gone.
    fn restore_or_clear(&mut self, restored: Option<LargeState>) {
        match restored {
            Some(state) => self.restore_large_state(state),
            None => self.clear_large_content(),
        }
    }

    /// Drops the large content, leaving the document without a source.
    fn clear_large_content(&mut self) {
        self.table = None;
        self.clear_large_state();
        self.phase = DocumentPhase::Ready;
    }
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

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;
    use std::sync::mpsc;
    use std::time::Duration;

    use tabtool_core::CsvConfiguration;
    use tabtool_large::{CellOverlay, LargeCSVSource, LargeFileIndexer, MappedFile};

    use super::*;
    use crate::types::LargeStatus;

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
}
