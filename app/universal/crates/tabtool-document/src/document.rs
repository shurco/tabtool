//! The document value: table, phase, selection and save state.

use std::cell::Cell;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use tabtool_core::{CsvConfiguration, CsvTable, CsvWriteIssue, RowSource};

use crate::filter_model::RowFilterModel;
use crate::io;
use crate::large_save::LargeSaveJob;
use crate::large_state::LargeState;
use crate::snapshot::DocumentSnapshot;
use crate::storage::Storage;
use crate::types::{
    ActiveSort, DocumentError, DocumentNotice, DocumentPhase, GridSelection, LargeStatus,
};
use crate::undo::UndoEntry;

/// One CSV document.
///
/// The value owns the table and every piece of view-independent state. Loading,
/// snapshots, editing, structural changes, selections, undo and saving all land
/// here. Exactly one content mode is active: an in-memory table, a mapped
/// large-file state, or neither while nothing has loaded.
pub struct Document {
    pub(crate) phase: DocumentPhase,
    pub(crate) table: Option<Arc<CsvTable>>,
    /// The mapped state of a large file; `table` is `None` in this mode.
    pub(crate) large: Option<LargeState>,
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
        }
    }

    /// Replaces the document with a blank untitled table in `configuration`.
    ///
    /// The table is one row by three columns. When the configuration names the
    /// first row as headers, the three columns get empty headers and the data
    /// row stays, exactly as a new document in the Swift app.
    pub fn new_document(&mut self, configuration: CsvConfiguration) {
        let mut table = CsvTable::new_document(1, 3);
        if configuration.first_row_as_header {
            table.set_headers(Some(vec![String::new(); table.columns.len()]));
        }

        self.table = Some(Arc::new(table));
        self.clear_large_state();
        self.configuration = configuration;
        self.had_utf8_bom = false;
        self.file_path = None;
        self.selection = GridSelection::default();
        self.active_sort = None;
        self.row_filter = RowFilterModel::default();
        self.undo_entries.clear();
        self.redo_entries.clear();
        self.phase = DocumentPhase::Ready;
        self.bump_revision();
        self.refresh_phase();
    }

    /// Loads initial bytes and publishes the whole outcome at once.
    ///
    /// The UI may call this from a background thread. `configuration` is
    /// `None` to detect the format from the bytes, or `Some` to parse with an
    /// explicit format (including [`CsvConfiguration::default`], which must
    /// not trigger detection). `prefers_chinese` changes the detector's
    /// encoding order. A parse error leaves the table empty and reports
    /// [`DocumentPhase::ReadFailed`]; a completed document ignores a repeated
    /// load so edits cannot be replaced by a late duplicate.
    pub fn load_from_bytes(
        &mut self,
        bytes: &[u8],
        path: Option<&Path>,
        configuration: Option<CsvConfiguration>,
        prefers_chinese: bool,
    ) -> Result<(), DocumentError> {
        if matches!(
            self.phase,
            DocumentPhase::Ready | DocumentPhase::NeedsAttention(_)
        ) {
            return Ok(());
        }

        let configuration = configuration.unwrap_or_else(|| io::detect(bytes, prefers_chinese));
        self.file_path = path.map(Path::to_path_buf);
        self.clear_large_state();

        match io::parse(bytes, &configuration) {
            Ok(outcome) => {
                self.table = Some(Arc::new(outcome.table));
                self.configuration = outcome.configuration;
                self.had_utf8_bom = outcome.had_utf8_bom;
                self.phase = DocumentPhase::Ready;
                self.bump_revision();
                // Loading content is not an unsaved edit.
                self.saved_revision = self.revision;
                self.refresh_phase();
                self.refresh_filter();
                Ok(())
            }
            Err(error) => {
                self.configuration = configuration;
                self.table = None;
                self.phase = DocumentPhase::ReadFailed(error);
                self.bump_revision();
                Err(DocumentError::NotReady)
            }
        }
    }

    /// Reads and loads the file at `path` through `storage`.
    ///
    /// `configuration` and `prefers_chinese` are forwarded to
    /// [`Document::load_from_bytes`]. A read failure is distinct from a parse
    /// failure: the phase becomes [`DocumentPhase::UnreadableFile`] and the
    /// path stays bound for a retry.
    pub fn load_from_path(
        &mut self,
        path: &Path,
        storage: &dyn Storage,
        configuration: Option<CsvConfiguration>,
        prefers_chinese: bool,
    ) -> Result<(), DocumentError> {
        if matches!(
            self.phase,
            DocumentPhase::Ready | DocumentPhase::NeedsAttention(_)
        ) {
            return Ok(());
        }

        match storage.read(path) {
            Ok(bytes) => self.load_from_bytes(&bytes, Some(path), configuration, prefers_chinese),
            Err(_) => {
                self.clear_large_state();
                self.table = None;
                self.file_path = Some(path.to_path_buf());
                self.phase = DocumentPhase::UnreadableFile;
                self.bump_revision();
                Err(DocumentError::CannotReadFile)
            }
        }
    }

    /// Binds the document to a file, or makes it untitled with `None`.
    ///
    /// The table and format are left untouched.
    pub fn set_file_path(&mut self, path: Option<&Path>) {
        let path = path.map(Path::to_path_buf);
        if path == self.file_path {
            return;
        }
        self.file_path = path;
        self.bump_revision();
    }

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
    pub fn is_large_mode(&self) -> bool {
        self.large.is_some()
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

    /// Whether cells can be edited now: always in memory, and in large mode
    /// once the index is complete.
    pub fn can_edit_cells(&self) -> bool {
        match &self.large {
            Some(_) => self.large_status == LargeStatus::Ready,
            None => true,
        }
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
    pub(crate) fn clear_large_state(&mut self) {
        self.large = None;
        self.large_status = LargeStatus::None;
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
