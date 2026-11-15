//! Starting, waiting for and applying a streaming save on the document.

use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;

use tabtool_large::{LargeFileSaveError, PreparedSave, lock};

use super::{Inner, LargeSaveAdoption, LargeSaveJob, LargeSaveJobOutcome, Shared};
use crate::Document;
use crate::large_save_worker::{SaveInputs, WorkerOutcome, execute_save};
use crate::large_state::LargeState;
use crate::types::{DocumentPhase, LargeStatus};

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

    /// Whether the running large save has stopped, without joining it.
    ///
    /// This reports an early failure too, where the progress never reaches 1.0.
    #[must_use]
    pub fn large_save_finished(&self) -> bool {
        self.large_save
            .as_ref()
            .is_some_and(LargeSaveJob::is_finished)
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
    ///
    /// The file may have changed while the save ran without a prior staleness
    /// poll (the worker reports `SourceChanged` itself), so the restored state
    /// is checked immediately and the banner is raised before the caller sees
    /// `Ready`.
    pub(super) fn restore_large_state(&mut self, state: LargeState) {
        self.table = None;
        self.large = Some(state);
        self.large_status = LargeStatus::Ready;
        self.phase = DocumentPhase::Ready;
        self.refresh_large_staleness();
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
