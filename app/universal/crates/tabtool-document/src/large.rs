//! Large-file documents: mapping, indexing status, overlay edits and revert.
//!
//! Opening maps the file, checks or detects the format with the large-file
//! sniffer and starts the background indexer. The document never keeps a table
//! in this mode: cells, headers and staleness all come from the mapped source.
//! Edits go through [`tabtool_large::LargeCSVSource::set_cell`] and stay in its
//! overlay; the document records a targeted undo entry and counts the overlay
//! as its unsaved state.
//!
//! Structural commands stay inert because the crate-private
//! [`Document::can_edit_content`] requires an in-memory table. Filtering is
//! unavailable in large mode, matching the Swift document.

use std::path::Path;
use std::sync::Arc;

use tabtool_core::{ColumnId, CsvConfiguration, CsvError};
use tabtool_large::{
    CellOverlay, LargeCSVSource, LargeFileIndexer, MappedFile, Staleness, check, sniff,
};

use crate::Document;
use crate::filter_model::RowFilterModel;
use crate::large_state::{INDEX_PROGRESS_INTERVAL, INDEX_STEP_BYTES, LargeState};
use crate::storage::Storage;
use crate::types::{DocumentError, DocumentNotice, DocumentPhase, GridSelection, LargeStatus};

/// Why a file could not be mapped or sniffed.
enum MapFailure {
    /// The file bytes are unavailable.
    Unreadable,
    /// The file's format cannot be read in large mode.
    Csv(CsvError),
}

impl Document {
    /// Whether a file of this size opens through the memory-mapped path.
    ///
    /// A file at or above the threshold opens large; a smaller one stays in
    /// memory.
    pub fn uses_large_mode(file_size: u64, threshold_bytes: u64) -> bool {
        file_size >= threshold_bytes
    }

    /// Opens `path` large, detecting the format from its head unless
    /// `configuration` is supplied.
    ///
    /// The size is checked before anything is mapped, and the file is mapped
    /// and checked before any document state changes. A refused format leaves
    /// a document that already has content untouched and reports the typed
    /// error. A file smaller than `threshold_bytes` is refused with
    /// [`DocumentError::NotReady`] without touching the document; call
    /// [`Document::open_file`] to route by size instead. On success the table
    /// is dropped, the phase becomes [`DocumentPhase::Ready`] and the indexer
    /// starts on its worker thread. After a failed open of an empty document
    /// the large intent stays set, so a later format change reopens the file
    /// large instead of reading it into memory. A streaming save in flight
    /// reports [`DocumentError::Busy`].
    pub fn open_large(
        &mut self,
        path: &Path,
        configuration: Option<CsvConfiguration>,
        prefers_chinese: bool,
        threshold_bytes: u64,
    ) -> Result<(), DocumentError> {
        self.ensure_not_saving()?;
        let size = match std::fs::metadata(path) {
            Ok(metadata) => metadata.len(),
            Err(_) => {
                self.fail_large_open(path, DocumentPhase::UnreadableFile);
                return Err(DocumentError::CannotReadFile);
            }
        };
        if !Self::uses_large_mode(size, threshold_bytes) {
            return Err(DocumentError::NotReady);
        }
        match Self::map_large_file(path, configuration.as_ref(), prefers_chinese) {
            Ok((file, configuration)) => {
                if !Self::uses_large_mode(file.size() as u64, threshold_bytes) {
                    return Err(DocumentError::NotReady);
                }
                self.undo_entries.clear();
                self.redo_entries.clear();
                self.selection = GridSelection::default();
                self.row_filter = RowFilterModel::default();
                self.install_large(path, file, configuration, CellOverlay::new(), Vec::new());
                self.saved_revision = self.revision;
                Ok(())
            }
            Err(MapFailure::Unreadable) => {
                self.fail_large_open(path, DocumentPhase::UnreadableFile);
                Err(DocumentError::CannotReadFile)
            }
            Err(MapFailure::Csv(error)) => {
                self.fail_large_open(path, DocumentPhase::ReadFailed(error));
                Err(DocumentError::Csv(error))
            }
        }
    }

    /// Opens `path` through the size router.
    ///
    /// A file at or above `threshold_bytes` goes through [`Document::open_large`];
    /// a smaller file is read and parsed through `storage` as an in-memory
    /// document. When the size cannot be read, the in-memory path reports the
    /// typed read failure.
    pub fn open_file(
        &mut self,
        path: &Path,
        storage: &dyn Storage,
        configuration: Option<CsvConfiguration>,
        prefers_chinese: bool,
        threshold_bytes: u64,
    ) -> Result<(), DocumentError> {
        self.ensure_not_saving()?;
        let size = std::fs::metadata(path)
            .map(|metadata| metadata.len())
            .unwrap_or(0);
        if Self::uses_large_mode(size, threshold_bytes) {
            self.open_large(path, configuration, prefers_chinese, threshold_bytes)
        } else {
            self.load_from_path(path, storage, configuration, prefers_chinese)
        }
    }

    /// Re-opens the file with the current format, keeping the edits so far.
    ///
    /// Used after [`Document::refresh_large_staleness`] reports a changed file.
    /// The overlay, the undo history and the selection stay; the mapping, the
    /// index and the column identities are rebuilt. The path is mapped
    /// directly, so `storage` is only accepted for call-site symmetry with
    /// every other document open.
    pub fn reload_large(&mut self, _storage: &dyn Storage) -> Result<(), DocumentError> {
        self.ensure_not_saving()?;
        let Some(path) = self.file_path().map(Path::to_path_buf) else {
            return Err(DocumentError::NotReady);
        };
        let Some(large) = self.large.as_ref() else {
            return Err(DocumentError::NotReady);
        };
        let configuration = self.configuration.clone();
        let overlay = copy_overlay(&large.source.overlay);
        let file = Arc::new(MappedFile::open(&path).map_err(|_| DocumentError::CannotReadFile)?);
        check(&file, &configuration).map_err(DocumentError::Csv)?;
        self.install_large(&path, file, configuration, overlay, Vec::new());
        Ok(())
    }

    /// Re-reads the file with `new`, dropping edits and the undo history.
    ///
    /// A dirty overlay requires `force`; without it the call reports
    /// [`DocumentError::RequiresConfirmation`] and changes nothing. The format
    /// is checked against the file head before the document changes, so a
    /// refused format (including UTF-16) keeps the previous source and
    /// configuration.
    pub fn reopen_large(
        &mut self,
        new: CsvConfiguration,
        force: bool,
    ) -> Result<(), DocumentError> {
        self.ensure_not_saving()?;
        if self.large.is_none() {
            return Err(DocumentError::NotReady);
        }
        let Some(path) = self.file_path().map(Path::to_path_buf) else {
            return Err(DocumentError::NotReady);
        };
        if !force && self.has_unsaved_changes() {
            return Err(DocumentError::RequiresConfirmation);
        }
        if !self.begin_reread() {
            return Err(DocumentError::RequiresConfirmation);
        }
        let result = self.reopen_large_inner(&path, new);
        self.end_reread();
        result
    }

    /// Publishes the indexer's progress into [`Document::large_status`].
    ///
    /// While the scan runs the status is [`LargeStatus::Indexing`] with the
    /// indexer's fraction; once it finishes the status becomes
    /// [`LargeStatus::Ready`]. A stale or saving document keeps its status.
    /// Rows are readable through [`Document::row_source`] throughout, but
    /// [`Document::can_edit_cells`] only turns true when the index is complete.
    pub fn poll_large_indexing(&mut self) {
        if matches!(
            self.large_status,
            LargeStatus::Stale { .. } | LargeStatus::Saving(_)
        ) {
            return;
        }
        let Some(large) = self.large.as_ref() else {
            self.large_status = LargeStatus::None;
            return;
        };
        let status = if large.indexer.is_finished() {
            LargeStatus::Ready
        } else {
            LargeStatus::Indexing(large.source.indexing_fraction())
        };
        self.phase = DocumentPhase::Ready;
        self.large_status = status;
    }

    /// Marks the document stale when the file changed on disk since it was
    /// mapped.
    ///
    /// A file that only grew reports [`LargeStatus::Stale`] with
    /// `appended = true`; a replaced or truncated file reports it with
    /// `appended = false`. The phase stays [`DocumentPhase::Ready`]. Only a
    /// ready document is checked, so indexing progress is not overwritten.
    pub fn refresh_large_staleness(&mut self) {
        let Some(large) = self.large.as_ref() else {
            return;
        };
        if self.large_status != LargeStatus::Ready {
            return;
        }
        match large.source.staleness() {
            Staleness::Unchanged => {}
            Staleness::Appended => self.large_status = LargeStatus::Stale { appended: true },
            Staleness::ReplacedOrTruncated => {
                self.large_status = LargeStatus::Stale { appended: false };
            }
        }
    }

    /// Sets one cell through the source overlay and records its undo.
    ///
    /// The displayed text is compared first, so writing the same text is a
    /// no-op even while the index is incomplete. Any other edit is refused
    /// with [`CsvError::RowNotEditable`] while indexing, while a re-read owns
    /// the document, or while the file is stale; the source can also refuse
    /// with [`CsvError::AmbiguousOutput`] or [`CsvError::UnencodableText`].
    /// Every refusal leaves the overlay untouched and raises
    /// [`DocumentNotice::EditRejected`].
    pub fn set_large_cell(
        &mut self,
        text: &str,
        row: usize,
        column: usize,
    ) -> Result<(), CsvError> {
        let Some(large) = self.large.as_ref() else {
            return self.reject_large_edit(CsvError::RowNotEditable);
        };
        let previous = large.source.cell(row, column);
        if previous == text {
            return Ok(());
        }
        if !self.can_edit_cells() || self.rereading {
            return self.reject_large_edit(CsvError::RowNotEditable);
        }
        let outcome = large
            .source
            .set_cell(text, row, column)
            .map(|_| large.source.physical_row(row));
        match outcome {
            Ok(physical_row) => {
                self.record_large_cell_change(physical_row, column, previous, text.to_owned());
                self.bump_revision();
                Ok(())
            }
            Err(error) => self.reject_large_edit(error),
        }
    }

    /// Turns the first row into a header, or back into data.
    ///
    /// The file bytes and the overlay are untouched; only the source's row
    /// mapping and the configuration move. One [`crate::UndoAction::ToggleHeader`]
    /// step restores the previous mapping. A stale or indexing document
    /// refuses the toggle.
    pub fn set_large_header_enabled(&mut self, enabled: bool) {
        let previous = self.configuration.first_row_as_header;
        if !self.apply_large_header_value(enabled) {
            return;
        }
        self.record_large_header_change(previous);
    }

    /// Drops every overlay edit and the undo history. The file is not re-read.
    ///
    /// The header mapping and the configuration are left as they are, matching
    /// the Swift `revertLargeEdits`.
    pub fn revert_large_edits(&mut self) {
        let Some(large) = self.large.as_ref() else {
            return;
        };
        large.source.overlay.remove_all();
        self.undo_entries.clear();
        self.redo_entries.clear();
        self.bump_revision();
    }

    /// Applies one header value without recording undo; undo and redo use it.
    ///
    /// Returns `false` without changing anything when there is no large source,
    /// the index is not ready, a re-read owns the document, or the mapping
    /// already has the requested value.
    pub(crate) fn apply_large_header_value(&mut self, enabled: bool) -> bool {
        let Some(large) = self.large.as_ref() else {
            return false;
        };
        if self.large_status != LargeStatus::Ready || self.rereading {
            return false;
        }
        if self.configuration.first_row_as_header == enabled {
            return false;
        }
        large.source.set_header_enabled(enabled);
        self.configuration.first_row_as_header = enabled;
        self.bump_revision();
        self.trim_selection();
        true
    }

    /// Applies one targeted overlay edit at a physical row; undo and redo use it.
    ///
    /// The display row is derived from the current header mapping, so an edit
    /// recorded before a header toggle still restores the same file cell.
    pub(crate) fn apply_large_cell_at(
        &mut self,
        physical_row: usize,
        column: usize,
        text: &str,
    ) -> Result<(), CsvError> {
        let Some(large) = self.large.as_ref() else {
            return Err(CsvError::RowNotEditable);
        };
        if !self.can_edit_cells() || self.rereading {
            return Err(CsvError::RowNotEditable);
        }
        let display_row = physical_row.saturating_sub(large.source.physical_row(0));
        large.source.set_cell(text, display_row, column)?;
        self.bump_revision();
        Ok(())
    }

    /// Maps, checks or detects the format, and starts the source and indexer.
    fn install_large(
        &mut self,
        path: &Path,
        file: Arc<MappedFile>,
        configuration: CsvConfiguration,
        overlay: CellOverlay,
        column_ids: Vec<ColumnId>,
    ) {
        let indexer = Arc::new(LargeFileIndexer::new(
            Arc::clone(&file),
            configuration.clone(),
            INDEX_STEP_BYTES,
            INDEX_PROGRESS_INTERVAL,
        ));
        indexer.start(|_| {});
        let source = Arc::new(LargeCSVSource::new(
            Arc::clone(&file),
            configuration.clone(),
            Arc::clone(&indexer),
            overlay,
            column_ids,
        ));
        self.table = None;
        self.large = Some(LargeState {
            source,
            indexer,
            file,
        });
        self.large_intent = true;
        self.configuration = configuration;
        self.file_path = Some(path.to_path_buf());
        self.phase = DocumentPhase::Ready;
        self.active_sort = None;
        self.large_status = LargeStatus::None;
        self.bump_revision();
        self.poll_large_indexing();
    }

    /// Records a failed large open on a document that has nothing to preserve.
    ///
    /// A document that already shows a table or a large source keeps its
    /// content, path, phase and undo history; the caller only receives the
    /// typed error. An empty or still-loading document adopts the failure
    /// instead: the path is bound for a retry, the phase reports it, and the
    /// sticky large intent stays set so recovery never reads the file into
    /// memory.
    fn fail_large_open(&mut self, path: &Path, phase: DocumentPhase) {
        if self.has_content() && !matches!(self.phase, DocumentPhase::Loading) {
            return;
        }
        self.clear_large_state();
        self.table = None;
        self.large_intent = true;
        self.file_path = Some(path.to_path_buf());
        self.phase = phase;
        self.active_sort = None;
        self.selection = GridSelection::default();
        self.row_filter = RowFilterModel::default();
        self.undo_entries.clear();
        self.redo_entries.clear();
        self.bump_revision();
    }

    /// Maps and checks the new format, then installs it. The document changes
    /// only after both succeed.
    fn reopen_large_inner(
        &mut self,
        path: &Path,
        new: CsvConfiguration,
    ) -> Result<(), DocumentError> {
        let file = Arc::new(MappedFile::open(path).map_err(|_| DocumentError::CannotReadFile)?);
        check(&file, &new).map_err(DocumentError::Csv)?;
        self.undo_entries.clear();
        self.redo_entries.clear();
        self.install_large(path, file, new, CellOverlay::new(), Vec::new());
        self.saved_revision = self.revision;
        Ok(())
    }

    /// Records a refused edit as a one-shot notice and returns the error.
    fn reject_large_edit(&self, error: CsvError) -> Result<(), CsvError> {
        self.set_notice(DocumentNotice::EditRejected(error));
        Err(error)
    }

    /// Maps the file and checks or sniffs its format without changing the
    /// document.
    fn map_large_file(
        path: &Path,
        configuration: Option<&CsvConfiguration>,
        prefers_chinese: bool,
    ) -> Result<(Arc<MappedFile>, CsvConfiguration), MapFailure> {
        let file = Arc::new(MappedFile::open(path).map_err(|_| MapFailure::Unreadable)?);
        let configuration = match configuration {
            Some(configuration) => {
                check(&file, configuration).map_err(MapFailure::Csv)?;
                configuration.clone()
            }
            None => sniff(&file, prefers_chinese).map_err(MapFailure::Csv)?,
        };
        Ok((file, configuration))
    }
}

/// Copies an overlay into a new one so a reload can reuse the edits.
fn copy_overlay(overlay: &CellOverlay) -> CellOverlay {
    let copy = CellOverlay::new();
    for (key, text) in overlay.copy() {
        copy.set(text, key);
    }
    copy
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Duration;

    use tabtool_core::{CsvConfiguration, CsvError};
    use tabtool_large::{CellOverlay, LargeCSVSource, LargeFileIndexer, MappedFile};

    use super::*;
    use crate::types::DocumentNotice;

    /// A document whose indexer was stopped after its first step, so the
    /// partial-index behavior is deterministic.
    fn partially_indexed_document(rows: usize) -> (tempfile::TempDir, Document) {
        let directory = tempfile::tempdir().expect("temporary directory");
        let path = directory.path().join("big.csv");
        let text = (0..rows)
            .map(|row| format!("r{row},v{row}"))
            .collect::<Vec<_>>()
            .join("\n");
        std::fs::write(&path, text).expect("fixture file is written");

        let file = Arc::new(MappedFile::open(&path).expect("fixture opens"));
        let configuration = CsvConfiguration::default();
        let indexer = Arc::new(LargeFileIndexer::new(
            Arc::clone(&file),
            configuration.clone(),
            4_096,
            Duration::ZERO,
        ));
        let stopper = Arc::clone(&indexer);
        indexer.start(move |progress| {
            if !progress.is_finished {
                stopper.cancel();
            }
        });
        indexer.wait();
        assert!(!indexer.is_finished());

        let source = Arc::new(LargeCSVSource::new(
            Arc::clone(&file),
            configuration,
            Arc::clone(&indexer),
            CellOverlay::new(),
            Vec::new(),
        ));
        let mut document = Document::new(CsvConfiguration::default());
        document.table = None;
        document.large = Some(LargeState {
            source,
            indexer,
            file,
        });
        document.file_path = Some(path);
        document.phase = DocumentPhase::Ready;
        document.bump_revision();
        document.saved_revision = document.revision;
        document.poll_large_indexing();
        (directory, document)
    }

    #[test]
    fn a_partial_index_shows_known_rows_and_refuses_edits() {
        let (_directory, mut document) = partially_indexed_document(20_000);
        let source = Arc::clone(&document.large_state().expect("large").source);

        assert!(source.row_count() > 0);
        assert!(source.row_count() < 20_000);
        assert_eq!(
            document.row_source().expect("source").row_count(),
            source.row_count()
        );
        assert_eq!(document.phase(), &DocumentPhase::Ready);
        assert!(matches!(
            document.large_status(),
            LargeStatus::Indexing(fraction) if fraction > 0.0 && fraction < 1.0
        ));
        assert!(!document.can_edit_cells());

        assert_eq!(
            document.set_large_cell("X", 0, 0),
            Err(CsvError::RowNotEditable)
        );
        assert_eq!(
            document.notice(),
            Some(DocumentNotice::EditRejected(CsvError::RowNotEditable))
        );
        assert!(!document.can_undo());
        assert!(source.overlay.is_empty());
    }

    #[test]
    fn a_same_text_edit_is_a_no_op_while_indexing() {
        let (_directory, mut document) = partially_indexed_document(20_000);

        assert_eq!(document.set_large_cell("r0", 0, 0), Ok(()));

        assert_eq!(document.notice(), None);
        assert!(!document.can_undo());
    }

    #[test]
    fn polling_an_in_memory_document_reports_no_large_status() {
        let mut document = Document::new(CsvConfiguration::default());

        document.poll_large_indexing();

        assert_eq!(document.large_status(), LargeStatus::None);
    }
}
