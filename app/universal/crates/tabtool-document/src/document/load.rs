//! Creating a document, loading its content and binding it to a file.

use std::path::Path;
use std::sync::Arc;

use tabtool_core::{CsvConfiguration, CsvTable};

use crate::Document;
use crate::filter_model::RowFilterModel;
use crate::io;
use crate::storage::Storage;
use crate::types::{DocumentError, DocumentPhase, GridSelection};

impl Document {
    /// Replaces the document with a blank untitled table in `configuration`.
    ///
    /// The table is one row by three columns. When the configuration names the
    /// first row as headers, the three columns get empty headers and the data
    /// row stays, exactly as a new document in the Swift app. A streaming save
    /// in flight owns the document, so this reports [`DocumentError::Busy`]
    /// until it finishes.
    pub fn new_document(&mut self, configuration: CsvConfiguration) -> Result<(), DocumentError> {
        self.ensure_not_saving()?;
        let mut table = CsvTable::new_document(1, 3);
        if configuration.first_row_as_header {
            table.set_headers(Some(vec![String::new(); table.columns.len()]));
        }

        self.table = Some(Arc::new(table));
        self.clear_large_state();
        self.large_intent = false;
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
        Ok(())
    }

    /// Loads initial bytes and publishes the whole outcome at once.
    ///
    /// The UI may call this from a background thread. `configuration` is
    /// `None` to detect the format from the bytes, or `Some` to parse with an
    /// explicit format (including [`CsvConfiguration::default`], which must
    /// not trigger detection). `prefers_chinese` changes the detector's
    /// encoding order. A parse error leaves the table empty and reports
    /// [`DocumentPhase::ReadFailed`]; a completed document ignores a repeated
    /// load so edits cannot be replaced by a late duplicate. A streaming save
    /// in flight reports [`DocumentError::Busy`] instead of replacing it.
    pub fn load_from_bytes(
        &mut self,
        bytes: &[u8],
        path: Option<&Path>,
        configuration: Option<CsvConfiguration>,
        prefers_chinese: bool,
    ) -> Result<(), DocumentError> {
        self.ensure_not_saving()?;
        if matches!(
            self.phase,
            DocumentPhase::Ready | DocumentPhase::NeedsAttention(_)
        ) {
            return Ok(());
        }

        let configuration = configuration.unwrap_or_else(|| io::detect(bytes, prefers_chinese));
        self.file_path = path.map(Path::to_path_buf);
        self.clear_large_state();
        self.large_intent = false;

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
        self.ensure_not_saving()?;
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
                self.large_intent = false;
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
    /// The table and format are left untouched; the binding is a revision
    /// change. A streaming save owns the document and a transactional re-read
    /// is replacing it, so binding a new path during either reports
    /// [`DocumentError::Busy`] or [`DocumentError::RequiresConfirmation`]
    /// instead of racing them.
    pub fn set_file_path(&mut self, path: Option<&Path>) -> Result<(), DocumentError> {
        self.ensure_not_saving()?;
        if self.rereading {
            return Err(DocumentError::RequiresConfirmation);
        }
        let path = path.map(Path::to_path_buf);
        if path == self.file_path {
            return Ok(());
        }
        self.file_path = path;
        self.bump_revision();
        Ok(())
    }
}
