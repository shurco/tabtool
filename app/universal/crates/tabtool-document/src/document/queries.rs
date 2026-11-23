//! Read access to the document state, its save snapshot and its notice.

use std::path::Path;
use std::sync::Arc;

use tabtool_core::{CsvConfiguration, CsvTable, CsvWriteIssue, RowSource};

use crate::Document;
use crate::large_state::LargeState;
use crate::snapshot::DocumentSnapshot;
use crate::types::{
    ActiveSort, DocumentError, DocumentNotice, DocumentPhase, GridSelection, LargeStatus,
};

impl Document {
    /// Captures the table, format and BOM state for saving.
    ///
    /// The table is shared, not copied, so repeated autosave attempts stay
    /// cheap. A document with no loaded table has nothing to save. A
    /// `NeedsAttention` document still returns its snapshot; the save path
    /// applies the write gate, not snapshotting.
    pub fn snapshot(&self) -> Result<DocumentSnapshot, DocumentError> {
        if matches!(
            self.phase,
            DocumentPhase::Loading | DocumentPhase::ReadFailed(_) | DocumentPhase::UnreadableFile
        ) {
            return Err(DocumentError::NotReady);
        }
        let table = self.table.as_ref().ok_or(DocumentError::NotReady)?;
        Ok(DocumentSnapshot {
            table: Arc::clone(table),
            configuration: self.configuration.clone(),
            includes_utf8_bom: self.had_utf8_bom,
            line_ending: self.line_ending,
        })
    }

    /// Serializes the current content with the same format a save would use.
    ///
    /// A document with no loaded table reports [`DocumentError::NotReady`];
    /// the writer reports the typed write gate as [`DocumentError::Csv`]
    /// (ambiguous no-quote output or text the configured encoding cannot
    /// represent).
    pub fn encoded_data(&self) -> Result<Vec<u8>, DocumentError> {
        let snapshot = self.snapshot()?;
        crate::snapshot::encode(&snapshot).map_err(DocumentError::Csv)
    }

    /// The in-memory table, or `None` while no table is loaded.
    pub fn table(&self) -> Option<&Arc<CsvTable>> {
        self.table.as_ref()
    }

    /// The format used to read and write the document.
    pub fn configuration(&self) -> &CsvConfiguration {
        &self.configuration
    }

    /// Why the document cannot currently be edited or saved.
    pub fn phase(&self) -> &DocumentPhase {
        &self.phase
    }

    /// The rows and columns selected in the grid.
    pub fn selection(&self) -> &GridSelection {
        &self.selection
    }

    /// The rows and columns selected in the grid, for mutation.
    pub fn selection_mut(&mut self) -> &mut GridSelection {
        &mut self.selection
    }

    /// The row sort currently applied, if any.
    pub fn active_sort(&self) -> Option<&ActiveSort> {
        self.active_sort.as_ref()
    }

    /// The file the document was loaded from or saved to, if any.
    pub fn file_path(&self) -> Option<&Path> {
        self.file_path.as_deref()
    }

    /// Counts every content or file-binding change.
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// Whether the content differs from the last saved revision.
    ///
    /// An untitled document has no file holding its content, so it stays dirty
    /// until Save As binds a path.
    ///
    /// In-memory content is revision-based: a save records
    /// `saved_revision = revision`, and any later content or binding change
    /// bumps the revision. Undoing an edit back to the saved text therefore
    /// still reports dirty. The file on disk is never re-read, so a change made
    /// outside the document is not detected; the UI tracks revisions
    /// separately for autosave and close prompts.
    ///
    /// A large document is dirty while its overlay holds an unsaved edit,
    /// matching the Swift `hasUnsavedLargeEdits`: an edit written back to the
    /// file's own text clears the overlay and the document becomes clean
    /// without a save.
    pub fn is_dirty(&self) -> bool {
        if let Some(large) = &self.large {
            return !large.source.overlay.is_empty();
        }
        self.file_path.is_none() || self.revision != self.saved_revision
    }

    /// The typed problem that blocks saving, when the phase reports one.
    pub fn write_issue(&self) -> Option<CsvWriteIssue> {
        match self.phase {
            DocumentPhase::NeedsAttention(issue) => Some(issue),
            _ => None,
        }
    }

    /// Where a large file stands.
    ///
    /// While a streaming save runs, the reported fraction is read from the
    /// job, so callers see the writer's progress without polling a setter.
    pub fn large_status(&self) -> LargeStatus {
        if let LargeStatus::Saving(_) = self.large_status
            && let Some(job) = &self.large_save
        {
            return LargeStatus::Saving(job.progress());
        }
        self.large_status
    }

    /// The mapped state of a large document, if this is one.
    pub fn large_state(&self) -> Option<&LargeState> {
        self.large.as_ref()
    }

    /// Whether this document opens its file through the memory-mapped path.
    ///
    /// The answer stays true after a refused or failed open and while a
    /// streaming save holds the mapping, so callers never route the file
    /// through the in-memory reader.
    pub fn is_large_mode(&self) -> bool {
        self.large_intent
    }

    /// Whether a table or a large-file source is loaded.
    pub fn has_content(&self) -> bool {
        self.table.is_some() || self.large.is_some()
    }

    /// The rows the grid and status bar read.
    ///
    /// A large document reads its mapped source; its rows appear as soon as
    /// the mapping exists, even while the index is still growing.
    pub fn row_source(&self) -> Option<&dyn RowSource> {
        if let Some(large) = &self.large {
            return Some(large.source.as_ref() as &dyn RowSource);
        }
        self.table.as_deref().map(|table| table as &dyn RowSource)
    }

    /// Whether cells can be edited now: in memory with a table, and in large
    /// mode once a source is installed and its index is complete. A failed
    /// large open leaves no source, so cells stay read-only. A streaming save
    /// owns the document while it runs, so no rows are available and the answer
    /// is `false`.
    pub fn can_edit_cells(&self) -> bool {
        if self.large_save.is_some() || matches!(self.large_status, LargeStatus::Saving(_)) {
            return false;
        }
        match &self.large {
            Some(_) => self.large_status == LargeStatus::Ready,
            None => !self.large_intent && self.table.is_some(),
        }
    }

    /// Whether rows and columns can be added, removed, moved, sorted or
    /// renamed.
    ///
    /// Those commands need the in-memory table. Large files edit cells through
    /// their overlay; a filtered view locks row structure separately through
    /// [`Document::can_edit_row_structure`].
    pub fn can_edit_structure(&self) -> bool {
        !self.is_large_mode() && self.can_edit_content()
    }

    /// Whether Select All can represent every row.
    ///
    /// Large files cannot: [`GridSelection`] holds one index per row, and a
    /// multi-gigabyte file would need millions of entries. A large document
    /// deliberately reports `false`; the UI disables the command instead of
    /// freezing or running out of memory (design §14.18, approval pending).
    pub fn can_select_all(&self) -> bool {
        !self.is_large_mode()
    }

    /// The one-shot message the window should present, if any.
    ///
    /// A refused large-file edit raises [`DocumentNotice::EditRejected`].
    /// The copy commands raise [`DocumentNotice::CopyLimitExceeded`] when a
    /// selection would exceed [`tabtool_large::LargeFile::MAX_COPY_CELLS`];
    /// the window clears the notice by presenting it.
    pub fn notice(&self) -> Option<DocumentNotice> {
        self.notice.get()
    }

    /// Records a notice for the window to present.
    pub(crate) fn set_notice(&self, notice: DocumentNotice) {
        self.notice.set(Some(notice));
    }

    /// Takes the one-shot message the window should present, if any.
    ///
    /// A taken notice is gone; a second take returns `None`.
    pub fn take_notice(&mut self) -> Option<DocumentNotice> {
        self.notice.take()
    }

    /// Pins a saving state for deterministic UI snapshots.
    ///
    /// Test-only: no worker runs and no file is written. The status reports
    /// the given fraction until the fixture is dropped.
    #[cfg(feature = "test-support")]
    #[doc(hidden)]
    pub fn force_saving_state_for_test(&mut self, fraction: f64) {
        self.table = None;
        self.large = None;
        self.large_save = None;
        self.large_intent = true;
        self.phase = DocumentPhase::Ready;
        self.large_status = LargeStatus::Saving(fraction.clamp(0.0, 1.0));
        self.bump_revision();
    }

    /// Pins the indexing progress for deterministic UI snapshots.
    ///
    /// Test-only: the mapping and row source stay real, so the grid renders
    /// while the status line and large-file banner show a stable fraction.
    #[cfg(feature = "test-support")]
    #[doc(hidden)]
    pub fn force_indexing_state_for_test(&mut self, fraction: f64) {
        self.pinned_indexing = Some(fraction.clamp(0.0, 1.0));
    }
}
