//! The document value: table, phase, selection and save state.

use std::cell::Cell;
use std::path::PathBuf;
use std::sync::Arc;

use tabtool_core::{CsvConfiguration, CsvTable};

use crate::filter_model::RowFilterModel;
use crate::large_save::LargeSaveJob;
use crate::large_state::LargeState;
use crate::snapshot::DocumentSnapshot;
use crate::types::{
    ActiveSort, DocumentError, DocumentNotice, DocumentPhase, GridSelection, LargeStatus,
};
use crate::undo::UndoEntry;

mod load;
mod queries;

/// One CSV document.
///
/// The value owns the table and every piece of view-independent state. Loading,
/// snapshots, editing, structural changes, selections, undo and saving all land
/// here. Exactly one content mode is active: an in-memory table, a mapped
/// large-file state, or neither while nothing has loaded.
///
/// A document routed to the large path keeps that intent even when no source is
/// installed (a refused format or an unreadable file). Recovery then goes
/// through a format change that reopens the file large; it must never fall back
/// to reading a multi-gigabyte file into memory.
///
/// Dropping a document does not cancel or join a streaming save. A caller that
/// owns a document with a save in flight must cancel and wait before dropping
/// it; the close flow in Plan 4 owns that order.
pub struct Document {
    pub(crate) phase: DocumentPhase,
    pub(crate) table: Option<Arc<CsvTable>>,
    /// The mapped state of a large file; `table` is `None` in this mode.
    pub(crate) large: Option<LargeState>,
    /// Whether this document took, or was routed to take, the large-file path.
    ///
    /// Sticky across a failed open and a streaming save. It selects the large
    /// reopen path and keeps cells read-only when no source was installed.
    pub(crate) large_intent: bool,
    pub(crate) configuration: CsvConfiguration,
    pub(crate) selection: GridSelection,
    pub(crate) active_sort: Option<ActiveSort>,
    pub(crate) row_filter: RowFilterModel,
    pub(crate) undo_entries: Vec<UndoEntry>,
    pub(crate) redo_entries: Vec<UndoEntry>,
    pub(crate) file_path: Option<PathBuf>,
    /// Whether the loaded bytes began with a UTF-8 byte order mark.
    pub(crate) had_utf8_bom: bool,
    pub(crate) revision: u64,
    /// The revision the file last matched; a re-read from disk or a save
    /// refreshes it.
    pub(crate) saved_revision: u64,
    /// The snapshot captured for saving, keyed by the revision it describes.
    ///
    /// Saving reuses the entry while the revision is unchanged, so repeated
    /// saves of an untouched document do not clone the table metadata. Every
    /// revision bump clears it.
    pub(crate) snapshot_cache: Option<(u64, Arc<DocumentSnapshot>)>,
    /// Whether a transactional re-read currently owns the document.
    pub(crate) rereading: bool,
    /// The streaming save in flight, if any. Its shared state survives even
    /// when the caller drops the returned [`LargeSaveJob`].
    pub(crate) large_save: Option<LargeSaveJob>,
    /// A one-shot message the window should present.
    ///
    /// A [`Cell`] lets the read-only copy commands raise the copy-limit
    /// notice while the rest of the document stays borrowed immutably.
    notice: Cell<Option<DocumentNotice>>,
    pub(crate) large_status: LargeStatus,
    /// A pinned indexing fraction for deterministic UI snapshots.
    #[cfg(feature = "test-support")]
    pub(crate) pinned_indexing: Option<f64>,
}

impl Document {
    /// Creates an untitled document with a blank 1×3 table.
    ///
    /// The phase stays [`DocumentPhase::Loading`] until a load materializes
    /// file-backed content.
    pub fn new(configuration: CsvConfiguration) -> Self {
        Self {
            phase: DocumentPhase::Loading,
            // The placeholder table keeps the untitled defaults observable
            // before a load; `new_document` replaces it with loaded state.
            table: Some(Arc::new(CsvTable::new_document(1, 3))),
            large: None,
            large_intent: false,
            configuration,
            selection: GridSelection::default(),
            active_sort: None,
            row_filter: RowFilterModel::default(),
            undo_entries: Vec::new(),
            redo_entries: Vec::new(),
            file_path: None,
            had_utf8_bom: false,
            revision: 0,
            // An untitled document reports dirty until Save As binds a path;
            // loading file content resets this to the loaded revision.
            saved_revision: 0,
            snapshot_cache: None,
            rereading: false,
            large_save: None,
            notice: Cell::new(None),
            large_status: LargeStatus::None,
            #[cfg(feature = "test-support")]
            pinned_indexing: None,
        }
    }

    /// Recomputes `Ready`/`NeedsAttention` from the current content and format.
    ///
    /// A large document is always [`DocumentPhase::Ready`] while its source is
    /// available: large files have no in-memory table to validate and their
    /// write gates belong to the streaming saver. The save gate of an
    /// in-memory document must match [`Document::encoded_data`], so headers are
    /// validated only when the format writes them. A document without content
    /// keeps its current phase. Tasks that replace content or format call this
    /// so the phase never goes stale.
    pub(crate) fn refresh_phase(&mut self) {
        if self.large.is_some() {
            self.phase = DocumentPhase::Ready;
            return;
        }
        if self.table.is_some() {
            self.refresh_write_issue();
        }
    }

    /// Drops the mapped state and its status, leaving the in-memory mode.
    ///
    /// The sticky [`Document::is_large_mode`] intent is not cleared here; the
    /// sites that switch to a new in-memory document reset it themselves, and a
    /// failed large open keeps it.
    pub(crate) fn clear_large_state(&mut self) {
        self.large = None;
        self.large_status = LargeStatus::None;
    }

    /// Refuses an operation that would replace the document while a streaming
    /// save owns it.
    ///
    /// The save keeps the original state out of the document until it settles,
    /// so letting another operation open, load or revert would publish a
    /// second state that the save outcome then overwrites.
    pub(crate) fn ensure_not_saving(&self) -> Result<(), DocumentError> {
        if self.large_save.is_some() || matches!(self.large_status, LargeStatus::Saving(_)) {
            return Err(DocumentError::Busy);
        }
        Ok(())
    }

    /// Counts one content or file-binding change.
    ///
    /// The save-snapshot cache is invalidated here, so every revision bump
    /// drops the value a later save would otherwise reuse.
    pub(crate) fn bump_revision(&mut self) {
        self.revision += 1;
        self.snapshot_cache = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Opens a small file through the large path so the mode flag is real.
    fn large_document() -> Document {
        let directory = tempfile::tempdir().expect("temporary directory");
        let path = directory.path().join("big.csv");
        std::fs::write(&path, "a\nb\n").expect("fixture file is written");
        let mut document = Document::new(CsvConfiguration::default());
        document
            .open_large(&path, None, false, 0)
            .expect("open the fixture large");
        document.large_state().expect("large state").indexer.wait();
        document.poll_large_indexing();
        document
    }

    #[test]
    fn large_documents_cannot_filter_or_toggle_the_bar() {
        let mut document = large_document();
        assert!(document.table().is_none());
        assert!(document.is_large_mode());
        assert_eq!(document.phase(), &DocumentPhase::Ready);

        assert!(!document.can_filter());

        document.toggle_filter_bar();
        assert!(!document.row_filter().is_bar_visible);
    }

    #[test]
    fn a_loading_document_cannot_filter() {
        let document = Document::new(CsvConfiguration::default());
        assert!(!document.can_filter());
        assert!(!document.can_rename_column(0));
    }
}
