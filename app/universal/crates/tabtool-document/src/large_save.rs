//! Background streaming save for large documents.
//!
//! [`Document::start_large_save`] takes the mapped state out of the document,
//! captures a [`tabtool_large::LargeSaveSnapshot`] and runs
//! [`tabtool_large::save`] on one worker thread. [`LargeSaveJob`] shares its
//! progress and cancellation flag with the document, and [`LargeSaveJob::wait`]
//! joins the worker and reports how it ended.
//!
//! A cancelled or failed save puts the original [`LargeState`] back, so the
//! mapping, the overlay edits and the undo history stay usable. A successful
//! save only reports the written destination; adopting the written file is
//! Task 9b. Until then a successful in-place save leaves the document without
//! large content because the old mapping no longer matches the file on disk,
//! while a save to a new path keeps the untouched original state.

use std::fmt;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread::{self, JoinHandle};

use tabtool_large::{LargeFileSaveError, Staleness, volume_capacity};

use crate::Document;
use crate::large_state::LargeState;
use crate::types::{DocumentPhase, LargeStatus};

/// How a large save ended.
#[derive(Debug)]
pub enum LargeSaveJobOutcome {
    /// The file was written. Task 9b adopts it into the document.
    Saved {
        /// The path the streaming writer wrote.
        destination: PathBuf,
    },
    /// The save was cancelled; the original state was restored.
    Cancelled,
    /// The save failed; the original state was restored.
    Failed(LargeFileSaveError),
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
    /// The first waiter joins the thread and receives the real error from a
    /// failed save. A later waiter sees the cached outcome; a failed save is
    /// then reported with a generic error because the first error value was
    /// already handed out.
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
    destination: PathBuf,
    inner: Mutex<Inner>,
}

/// The worker handle, the state to restore, and the cached outcome.
struct Inner {
    handle: Option<JoinHandle<Result<(), LargeFileSaveError>>>,
    original: Option<LargeState>,
    restored: Option<LargeState>,
    outcome: Option<OutcomeKind>,
}

/// The kind of a finished save, kept for a second waiter.
#[derive(Clone, Copy)]
enum OutcomeKind {
    Saved,
    Cancelled,
    Failed,
}

impl Document {
    /// Starts writing a large document with its edits.
    ///
    /// `destination` is Save As; `None` writes the current file. The document
    /// must be large, ready and fully indexed. The mapped state leaves the
    /// document while the worker runs, so cells cannot be edited and
    /// [`Document::large_status`] reports [`LargeStatus::Saving`]. A save
    /// without a destination and without overlay edits does no work.
    ///
    /// A second start while a save runs is refused, like a stale file
    /// ([`LargeFileSaveError::SourceChanged`]) or a document that is not ready
    /// ([`LargeFileSaveError::IndexingNotFinished`]).
    pub fn start_large_save(
        &mut self,
        destination: Option<&Path>,
    ) -> Result<LargeSaveJob, LargeFileSaveError> {
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
        let target = destination
            .map(Path::to_path_buf)
            .or_else(|| self.file_path.clone())
            .ok_or(LargeFileSaveError::IndexingNotFinished)?;
        let snapshot = state.source.save_snapshot();
        let should_write = destination.is_some() || !snapshot.edits.is_empty();
        let state = self.large.take().expect("the state was checked above");

        self.table = None;
        self.large_status = LargeStatus::Saving(0.0);

        let progress = AtomicU64::new(0.0f64.to_bits());
        let cancel = AtomicBool::new(false);
        let shared = Arc::new(Shared {
            progress,
            cancel,
            destination: target.clone(),
            inner: Mutex::new(Inner {
                handle: None,
                original: Some(state),
                restored: None,
                outcome: None,
            }),
        });

        let worker_shared = Arc::clone(&shared);
        let spawned = thread::Builder::new()
            .name("tabtool-large-save".to_owned())
            .spawn(move || {
                if !should_write {
                    worker_shared
                        .progress
                        .store(1.0f64.to_bits(), Ordering::Release);
                    return Ok(());
                }
                if worker_shared.cancel.load(Ordering::Acquire) {
                    return Err(cancelled_error());
                }
                tabtool_large::save(
                    snapshot,
                    &target,
                    volume_capacity,
                    |fraction| {
                        worker_shared
                            .progress
                            .store(fraction.to_bits(), Ordering::Release)
                    },
                    &worker_shared.cancel,
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
                let state = lock(&shared.inner).original.take();
                match state {
                    Some(state) => self.restore_large_state(state),
                    None => self.large_status = LargeStatus::Ready,
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

    /// Waits for the save in flight and applies its outcome.
    ///
    /// Cancelled and failed saves restore the original mapping, edits and undo
    /// history; a failure returns its error. A successful save keeps the
    /// original state only when it still matches the file, as after a no-edit
    /// save or Save As. A successful in-place save drops the stale mapping and
    /// leaves [`LargeStatus::None`]; Task 9b adopts the written file.
    pub fn wait_for_large_save(&mut self) -> Result<(), LargeFileSaveError> {
        let Some(job) = self.large_save.take() else {
            return Ok(());
        };
        let shared = Arc::clone(&job.shared);
        let outcome = job.wait();
        let restored = lock(&shared.inner).restored.take();
        match outcome {
            LargeSaveJobOutcome::Saved { .. } => {
                match restored {
                    Some(state) if state.source.staleness() == Staleness::Unchanged => {
                        self.restore_large_state(state);
                    }
                    _ => self.clear_large_content(),
                }
                Ok(())
            }
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
    let result = handle.join().unwrap_or_else(|_| {
        Err(LargeFileSaveError::Io(io::Error::other(
            "the large save thread panicked",
        )))
    });
    let kind = match &result {
        Ok(()) => OutcomeKind::Saved,
        Err(_) if is_cancelled(shared, &result) => OutcomeKind::Cancelled,
        Err(_) => OutcomeKind::Failed,
    };
    {
        let mut inner = lock(&shared.inner);
        inner.outcome = Some(kind);
        inner.restored = inner.original.take();
    }
    match result {
        Ok(()) => LargeSaveJobOutcome::Saved {
            destination: shared.destination.clone(),
        },
        Err(_) if matches!(kind, OutcomeKind::Cancelled) => LargeSaveJobOutcome::Cancelled,
        Err(error) => LargeSaveJobOutcome::Failed(error),
    }
}

/// The outcome of a save another handle already joined.
fn cached_outcome(shared: &Shared) -> LargeSaveJobOutcome {
    loop {
        let kind = lock(&shared.inner).outcome;
        if let Some(kind) = kind {
            return match kind {
                OutcomeKind::Saved => LargeSaveJobOutcome::Saved {
                    destination: shared.destination.clone(),
                },
                OutcomeKind::Cancelled => LargeSaveJobOutcome::Cancelled,
                OutcomeKind::Failed => LargeSaveJobOutcome::Failed(LargeFileSaveError::Io(
                    io::Error::other("the large save failed before this waiter joined"),
                )),
            };
        }
        thread::yield_now();
    }
}

/// Whether a failure is the writer answering the cancel flag.
fn is_cancelled(shared: &Shared, result: &Result<(), LargeFileSaveError>) -> bool {
    shared.cancel.load(Ordering::Acquire)
        && matches!(
            result,
            Err(LargeFileSaveError::Io(error)) if error.kind() == io::ErrorKind::Interrupted
        )
}

fn cancelled_error() -> LargeFileSaveError {
    LargeFileSaveError::Io(io::Error::new(io::ErrorKind::Interrupted, "save cancelled"))
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}
