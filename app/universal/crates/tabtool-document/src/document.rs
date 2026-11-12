//! The document value: table, phase, selection and save state.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use tabtool_core::{
    CsvConfiguration, CsvError, CsvTable, CsvWriteIssue, CsvWriter, RowSource,
    issue as validate_write,
};

use crate::io;
use crate::snapshot::DocumentSnapshot;
use crate::storage::Storage;
use crate::types::{
    ActiveSort, DocumentError, DocumentNotice, DocumentPhase, GridSelection, LargeStatus,
};

/// One CSV document.
///
/// The value owns the table and every piece of view-independent state. Loading,
/// snapshots and the save gate land here; editing lands in later tasks and
/// builds on the same state.
pub struct Document {
    phase: DocumentPhase,
    table: Option<Arc<CsvTable>>,
    /// Placeholder for the memory-mapped state of a large file.
    ///
    /// The large-file task replaces the unit value with the mapped source,
    /// indexer and file handle.
    large: Option<()>,
    configuration: CsvConfiguration,
    selection: GridSelection,
    active_sort: Option<ActiveSort>,
    /// Placeholder for the row filter; the filter task adds the model.
    #[allow(dead_code)]
    filter: (),
    /// Placeholder for the undo history; the editing task adds the stack.
    #[allow(dead_code)]
    undo: (),
    file_path: Option<PathBuf>,
    /// Whether the loaded bytes began with a UTF-8 byte order mark.
    had_utf8_bom: bool,
    revision: u64,
    saved_revision: u64,
    /// A one-shot message the window should present.
    notice: Option<DocumentNotice>,
    large_status: LargeStatus,
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
            filter: (),
            undo: (),
            file_path: None,
            had_utf8_bom: false,
            revision: 0,
            // Task 7 reconciles revisions with saves; an untitled document
            // reports clean until then.
            saved_revision: 0,
            notice: None,
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
        self.configuration = configuration;
        self.had_utf8_bom = false;
        self.file_path = None;
        self.selection = GridSelection::default();
        self.active_sort = None;
        self.phase = DocumentPhase::Ready;
        self.revision += 1;
        self.refresh_phase();
    }

    /// Loads initial bytes and publishes the whole outcome at once.
    ///
    /// The UI may call this from a background thread. A parse error leaves the
    /// table empty and reports [`DocumentPhase::ReadFailed`]; a completed
    /// document ignores a repeated load so edits cannot be replaced by a late
    /// duplicate. When the document has the default configuration the format is
    /// detected from the bytes, mirroring a Swift document opened without a
    /// supplied configuration; any other configuration is used verbatim.
    pub fn load_from_bytes(
        &mut self,
        bytes: &[u8],
        path: Option<&Path>,
    ) -> Result<(), DocumentError> {
        if matches!(
            self.phase,
            DocumentPhase::Ready | DocumentPhase::NeedsAttention(_)
        ) {
            return Ok(());
        }

        let configuration = if self.configuration == CsvConfiguration::default() {
            io::detect(bytes, false)
        } else {
            self.configuration.clone()
        };
        self.file_path = path.map(Path::to_path_buf);

        match io::parse(bytes, &configuration) {
            Ok(outcome) => {
                self.table = Some(Arc::new(outcome.table));
                self.configuration = outcome.configuration;
                self.had_utf8_bom = outcome.had_utf8_bom;
                self.phase = DocumentPhase::Ready;
                self.revision += 1;
                // Loading content is not an unsaved edit.
                self.saved_revision = self.revision;
                self.refresh_phase();
                Ok(())
            }
            Err(error) => {
                self.configuration = configuration;
                self.table = None;
                self.phase = DocumentPhase::ReadFailed(error);
                self.revision += 1;
                Err(DocumentError::NotReady)
            }
        }
    }

    /// Reads and loads the file at `path` through `storage`.
    ///
    /// A read failure is distinct from a parse failure: the phase becomes
    /// [`DocumentPhase::UnreadableFile`] and the path stays bound for a retry.
    pub fn load_from_path(
        &mut self,
        path: &Path,
        storage: &dyn Storage,
    ) -> Result<(), DocumentError> {
        if matches!(
            self.phase,
            DocumentPhase::Ready | DocumentPhase::NeedsAttention(_)
        ) {
            return Ok(());
        }

        match storage.read(path) {
            Ok(bytes) => self.load_from_bytes(&bytes, Some(path)),
            Err(_) => {
                self.table = None;
                self.file_path = Some(path.to_path_buf());
                self.phase = DocumentPhase::UnreadableFile;
                self.revision += 1;
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
        self.revision += 1;
    }

    /// Captures the table, format and BOM state for saving.
    ///
    /// The table is shared, not copied, so repeated autosave attempts stay
    /// cheap. A document with no loaded table has nothing to save.
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
    /// The writer reports the typed write gate: ambiguous no-quote output or
    /// text the configured encoding cannot represent. A document with no
    /// loaded table reports [`CsvError::CannotDecodeData`].
    pub fn encoded_data(&self) -> Result<Vec<u8>, CsvError> {
        let snapshot = self.snapshot().map_err(|_| CsvError::CannotDecodeData)?;
        let header = if snapshot.configuration.first_row_as_header {
            snapshot.table.headers()
        } else {
            None
        };
        let mut writer =
            CsvWriter::new(&snapshot.table, header.as_deref(), &snapshot.configuration);
        writer.include_utf8_bom = snapshot.includes_utf8_bom;
        writer.data()
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
    pub fn is_dirty(&self) -> bool {
        self.revision != self.saved_revision
    }

    /// The typed problem that blocks saving, when the phase reports one.
    pub fn write_issue(&self) -> Option<CsvWriteIssue> {
        match self.phase {
            DocumentPhase::NeedsAttention(issue) => Some(issue),
            _ => None,
        }
    }

    /// Where a large file stands.
    pub fn large_status(&self) -> LargeStatus {
        self.large_status
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
    pub fn row_source(&self) -> Option<&dyn RowSource> {
        self.table.as_deref().map(|table| table as &dyn RowSource)
    }

    /// Whether cells can be edited now: always in memory, and in large mode
    /// once the index is complete.
    pub fn can_edit_cells(&self) -> bool {
        !self.is_large_mode() || self.large_status == LargeStatus::Ready
    }

    /// The one-shot message the window should present, if any.
    ///
    /// Always `None` until the large-file tasks raise a notice; kept now so
    /// the window has one accessor for it.
    pub fn notice(&self) -> Option<DocumentNotice> {
        self.notice
    }

    /// Recomputes `Ready`/`NeedsAttention` from the current table and format.
    ///
    /// The save gate must match [`Document::encoded_data`], so headers are
    /// validated only when the format writes them. A document without a table
    /// keeps its current phase.
    fn refresh_phase(&mut self) {
        let Some(table) = self.table.as_deref() else {
            return;
        };
        let header = if self.configuration.first_row_as_header {
            table.headers()
        } else {
            None
        };
        self.phase = match validate_write(table, header.as_deref(), &self.configuration) {
            Some(issue) => DocumentPhase::NeedsAttention(issue),
            None => DocumentPhase::Ready,
        };
    }
}
