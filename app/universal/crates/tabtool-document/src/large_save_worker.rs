//! One large save on its worker: prepare, release the mapping, install, adopt
//! or restore.
//!
//! The original mapping is dropped after [`LargeFileSaver::prepare`] and before
//! [`tabtool_large::PreparedSave::install`]; Windows cannot replace a file while
//! a mapping of it is alive. Only the immutable data needed to rebuild the
//! source (index, overlay text, column ids, path and format) survives the swap.

use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use tabtool_core::{ColumnId, CsvConfiguration};
use tabtool_large::{
    CellKey, CellOverlay, LargeCSVSource, LargeFileIndexer, LargeFileSaveError, LargeFileSaver,
    LargeSaveSnapshot, MappedFile, RowIndex, volume_capacity,
};

use crate::large_save::{LargeSaveAdoption, LargeSaveJobOutcome};
use crate::large_state::{INDEX_PROGRESS_INTERVAL, INDEX_STEP_BYTES, LargeState};

/// Everything one worker owns from the moment the save starts.
///
/// The source and its indexer are kept only until `prepare` has read the file;
/// the immutable restore data is copied out before they are dropped.
pub(crate) struct SaveInputs {
    /// The mapped state taken out of the document.
    pub(crate) original: LargeState,
    /// The file, index, format and edits captured at the start.
    pub(crate) snapshot: LargeSaveSnapshot,
    /// The path the written file goes to (the source path for Save).
    pub(crate) target: PathBuf,
    /// The path holding the original mapping, for a restore after a failure.
    pub(crate) source_path: PathBuf,
    /// The format the source was opened with.
    pub(crate) configuration: CsvConfiguration,
    /// The column identities the grid keeps across the save.
    pub(crate) column_ids: Vec<ColumnId>,
    /// The overlay text at the start, for an install failure and for adoption.
    pub(crate) overlay: HashMap<CellKey, String>,
    /// The edits the writer saved, removed from the adopted overlay.
    pub(crate) saved_edits: HashMap<CellKey, String>,
    /// Whether a file has to be written; a plain save without edits is a no-op.
    pub(crate) should_write: bool,
}

/// How one worker ended, plus the original state to put back on failure.
pub(crate) struct WorkerOutcome {
    pub(crate) outcome: LargeSaveJobOutcome,
    /// The original mapping rebuilt from the seeded index, when a restore is
    /// possible.
    pub(crate) restored: Option<LargeState>,
}

/// Runs one save to completion.
///
/// `before_install` runs after every reference to the original mapping has been
/// dropped and before the target is replaced. It exists so a unit test can
/// assert the Windows constraint without a Windows build.
pub(crate) fn execute_save(
    inputs: SaveInputs,
    cancelled: &AtomicBool,
    progress: impl Fn(f64),
    before_install: impl FnOnce(),
) -> WorkerOutcome {
    let SaveInputs {
        original,
        snapshot,
        target,
        source_path,
        configuration,
        column_ids,
        overlay,
        saved_edits,
        should_write,
    } = inputs;

    if !should_write {
        // A plain save without edits never writes; the same mapping stays.
        progress(1.0);
        let file = Arc::clone(&original.file);
        let indexer = LargeFileIndexer::seeded(
            Arc::clone(&file),
            configuration.clone(),
            snapshot.index.clone(),
        );
        let source = Arc::new(LargeCSVSource::new(
            Arc::clone(&file),
            configuration,
            Arc::clone(&indexer),
            overlay_from(&overlay),
            column_ids,
        ));
        return WorkerOutcome {
            outcome: LargeSaveJobOutcome::Saved(LargeSaveAdoption {
                file,
                indexer,
                source,
            }),
            restored: None,
        };
    }

    let prepared =
        match LargeFileSaver::prepare(&snapshot, &target, volume_capacity, progress, cancelled) {
            Ok(prepared) => prepared,
            Err(error) => {
                return WorkerOutcome {
                    outcome: failure_outcome(error, cancelled),
                    restored: Some(original),
                };
            }
        };

    let restore = RestoreData {
        index: snapshot.index.clone(),
        configuration: configuration.clone(),
        overlay: overlay_from(&overlay),
        column_ids: column_ids.clone(),
        path: source_path,
    };

    // Windows rejects replacing a target while any mapping of it is alive.
    // Drop the snapshot's mapping too: it is the same file for a plain save.
    drop(snapshot);
    drop(original);
    before_install();

    match prepared.install() {
        Ok(()) => match index_target(&target, &configuration) {
            Ok((file, indexer)) => {
                let source = Arc::new(LargeCSVSource::new(
                    Arc::clone(&file),
                    configuration,
                    Arc::clone(&indexer),
                    adopted_overlay(&overlay, &saved_edits),
                    column_ids,
                ));
                WorkerOutcome {
                    outcome: LargeSaveJobOutcome::Saved(LargeSaveAdoption {
                        file,
                        indexer,
                        source,
                    }),
                    restored: None,
                }
            }
            Err(error) => WorkerOutcome {
                outcome: LargeSaveJobOutcome::Failed(LargeFileSaveError::Io(error)),
                // The target already holds the new bytes; rebuilding the old
                // state would pair the new file with the old index.
                restored: None,
            },
        },
        Err(error) => WorkerOutcome {
            outcome: failure_outcome(error, cancelled),
            restored: restore.remap(),
        },
    }
}

/// The immutable state needed to rebuild the original source without a scan.
struct RestoreData {
    index: RowIndex,
    configuration: CsvConfiguration,
    overlay: CellOverlay,
    column_ids: Vec<ColumnId>,
    path: PathBuf,
}

impl RestoreData {
    /// Maps the original path again and serves it through the saved index.
    fn remap(self) -> Option<LargeState> {
        let file = Arc::new(MappedFile::open(&self.path).ok()?);
        let indexer =
            LargeFileIndexer::seeded(Arc::clone(&file), self.configuration.clone(), self.index);
        let source = Arc::new(LargeCSVSource::new(
            Arc::clone(&file),
            self.configuration,
            Arc::clone(&indexer),
            self.overlay,
            self.column_ids,
        ));
        Some(LargeState {
            source,
            indexer,
            file,
        })
    }
}

/// Maps and fully indexes the file the writer just installed.
fn index_target(
    path: &Path,
    configuration: &CsvConfiguration,
) -> io::Result<(Arc<MappedFile>, Arc<LargeFileIndexer>)> {
    let file = Arc::new(MappedFile::open(path)?);
    let indexer = Arc::new(LargeFileIndexer::new(
        Arc::clone(&file),
        configuration.clone(),
        INDEX_STEP_BYTES,
        INDEX_PROGRESS_INTERVAL,
    ));
    indexer.start(|_| {});
    indexer.wait();
    Ok((file, indexer))
}

/// The overlay the adopted source starts with: every saved edit is gone from
/// the file's bytes and from the overlay.
fn adopted_overlay(
    overlay: &HashMap<CellKey, String>,
    saved: &HashMap<CellKey, String>,
) -> CellOverlay {
    let adopted = CellOverlay::new();
    for (key, text) in overlay {
        if saved.get(key) != Some(text) {
            adopted.set(text.clone(), *key);
        }
    }
    adopted
}

/// A fresh overlay carrying the given text.
fn overlay_from(overlay: &HashMap<CellKey, String>) -> CellOverlay {
    let copy = CellOverlay::new();
    for (key, text) in overlay {
        copy.set(text.clone(), *key);
    }
    copy
}

/// A failed save is a cancellation only when the caller asked and the writer
/// answered with its interrupted error.
fn failure_outcome(error: LargeFileSaveError, cancelled: &AtomicBool) -> LargeSaveJobOutcome {
    if cancelled.load(Ordering::Acquire)
        && matches!(&error, LargeFileSaveError::Io(io_error)
            if io_error.kind() == io::ErrorKind::Interrupted)
    {
        LargeSaveJobOutcome::Cancelled
    } else {
        LargeSaveJobOutcome::Failed(error)
    }
}
