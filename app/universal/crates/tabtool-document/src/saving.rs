//! Saving a loaded document: the write gate, atomic replace and revisions.
//!
//! Save is the only content write the document performs. It validates the
//! current table and format first, then serializes a cached
//! [`DocumentSnapshot`] and hands the bytes to [`Storage::write_atomic`]. A
//! failed gate or a failed write leaves the previous file and the dirty state
//! untouched. Large files use their own streaming saver and never reach this
//! path.

use std::path::Path;
use std::sync::Arc;

use tabtool_core::{CsvError, CsvWriteIssue};

use crate::Document;
use crate::snapshot::{DocumentSnapshot, encode};
use crate::storage::Storage;
use crate::types::{DocumentError, DocumentPhase};

/// Maps a write-validation issue to the typed error a save reports.
pub(crate) fn error_for_issue(issue: CsvWriteIssue) -> CsvError {
    match issue {
        CsvWriteIssue::UnencodableText => CsvError::UnencodableText,
        CsvWriteIssue::AmbiguousOutput => CsvError::AmbiguousOutput,
    }
}

impl Document {
    /// Whether Save would write now.
    ///
    /// A document still loading, failed to read, unreadable or without a table
    /// cannot save. Save As is available whenever the table itself is
    /// writable; Save additionally needs a file path. A re-read in flight owns
    /// the document, so saving is refused until it finishes.
    pub fn can_save(&self) -> bool {
        !self.rereading
            && self.file_path().is_some()
            && self.table.is_some()
            && matches!(
                self.phase,
                DocumentPhase::Ready | DocumentPhase::NeedsAttention(_)
            )
            && self.current_write_issue().is_none()
    }

    /// Writes the document to its current path.
    ///
    /// Returns [`DocumentError::RequiresConfirmation`] while a re-read is in
    /// flight, [`DocumentError::NotReady`] before a table loads or when the
    /// document has no path (Save As is the way to bind one), and
    /// [`DocumentError::Csv`] for the typed write issue. A blocked gate or a
    /// failed write leaves the document dirty and the previous file untouched.
    /// A streaming large save in flight reports [`DocumentError::Busy`].
    pub fn save(&mut self, storage: &dyn Storage) -> Result<(), DocumentError> {
        self.ensure_writable()?;
        let Some(path) = self.file_path().map(Path::to_path_buf) else {
            return Err(DocumentError::NotReady);
        };
        self.write_to(&path, storage, false)
    }

    /// Writes the document to `path`, then binds the document to it.
    ///
    /// The path changes only after the write succeeds, so a failed Save As
    /// leaves the previous target and the dirty state in place.
    pub fn save_as(&mut self, path: &Path, storage: &dyn Storage) -> Result<(), DocumentError> {
        self.ensure_writable()?;
        self.write_to(path, storage, true)
    }

    /// Recomputes the write gate from the current table and format.
    ///
    /// The phase becomes [`DocumentPhase::NeedsAttention`] with the issue, or
    /// [`DocumentPhase::Ready`] when the table is writable again. A document
    /// without a table keeps its current phase, and a document that is still
    /// loading keeps [`DocumentPhase::Loading`] so the placeholder table can
    /// never be published as saveable content.
    pub fn refresh_write_issue(&mut self) -> Option<CsvWriteIssue> {
        let issue = self.current_write_issue();
        if self.table.is_some() && !matches!(self.phase, DocumentPhase::Loading) {
            self.phase = match issue {
                Some(issue) => DocumentPhase::NeedsAttention(issue),
                None => DocumentPhase::Ready,
            };
        }
        issue
    }

    /// The typed CSV error the current content would hit, or `None`.
    ///
    /// This answers the write-gate question, not readiness: a document with no
    /// table has no typed CSV error even though saving reports
    /// [`DocumentError::NotReady`].
    pub fn save_error(&self) -> Option<CsvError> {
        self.current_write_issue().map(error_for_issue)
    }

    /// The write issue for the current table and format, if any.
    pub(crate) fn current_write_issue(&self) -> Option<CsvWriteIssue> {
        let table = self.table.as_deref()?;
        let header = if self.configuration.first_row_as_header {
            table.headers()
        } else {
            None
        };
        tabtool_core::issue(table, header.as_deref(), &self.configuration)
    }

    /// Rejects a save before any bytes are encoded or written.
    fn ensure_writable(&self) -> Result<(), DocumentError> {
        self.ensure_not_saving()?;
        if self.rereading {
            return Err(DocumentError::RequiresConfirmation);
        }
        if self.table.is_none()
            || !matches!(
                self.phase,
                DocumentPhase::Ready | DocumentPhase::NeedsAttention(_)
            )
        {
            return Err(DocumentError::NotReady);
        }
        match self.current_write_issue() {
            Some(issue) => Err(DocumentError::Csv(error_for_issue(issue))),
            None => Ok(()),
        }
    }

    /// Writes one snapshot and records the saved revision.
    ///
    /// `rebind` makes Save As adopt the target only after the write. Encoding
    /// happens before the write, so a gate failure never touches the file.
    fn write_to(
        &mut self,
        path: &Path,
        storage: &dyn Storage,
        rebind: bool,
    ) -> Result<(), DocumentError> {
        let snapshot = self.write_snapshot()?;
        let data = encode(&snapshot).map_err(DocumentError::Csv)?;
        storage
            .write_atomic(path, &data)
            .map_err(|_| DocumentError::CannotWriteFile)?;
        if rebind && self.file_path() != Some(path) {
            self.set_file_path(Some(path));
        }
        self.saved_revision = self.revision;
        // The gate passed above, but a stale phase must not keep reporting a
        // write issue after the bytes were written.
        self.refresh_phase();
        Ok(())
    }

    /// Returns the shared snapshot for the current revision.
    ///
    /// The snapshot is cached by revision so repeated saves of an unchanged
    /// document reuse the table Arc. Every revision bump clears the entry; the
    /// revision key also rejects a stale value.
    fn write_snapshot(&mut self) -> Result<Arc<DocumentSnapshot>, DocumentError> {
        if let Some((revision, snapshot)) = &self.snapshot_cache
            && *revision == self.revision
        {
            return Ok(Arc::clone(snapshot));
        }
        let snapshot = Arc::new(self.snapshot()?);
        self.snapshot_cache = Some((self.revision, Arc::clone(&snapshot)));
        Ok(snapshot)
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::sync::Arc;

    use tabtool_core::CsvConfiguration;

    use super::*;

    /// A ready file-backed document; the path never has to exist for these
    /// unit tests because no save is performed.
    fn document() -> Document {
        let configuration = CsvConfiguration::default();
        let mut document = Document::new(configuration.clone());
        document
            .load_from_bytes(
                b"a\n",
                Some(Path::new("fixture.csv")),
                Some(configuration),
                false,
            )
            .expect("load the fixture");
        document
    }

    #[test]
    fn a_loading_document_does_not_publish_readiness() {
        let mut document = Document::new(CsvConfiguration::default());
        assert_eq!(document.phase(), &DocumentPhase::Loading);

        assert_eq!(document.refresh_write_issue(), None);

        assert_eq!(document.phase(), &DocumentPhase::Loading);
    }

    #[test]
    fn repeated_snapshot_requests_at_one_revision_reuse_the_cached_arc() {
        let mut document = document();

        let first = document.write_snapshot().expect("first snapshot");
        let second = document.write_snapshot().expect("second snapshot");

        assert!(Arc::ptr_eq(&first, &second));
    }

    #[test]
    fn a_revision_bump_invalidates_the_cached_snapshot() {
        let mut document = document();
        let cached = document.write_snapshot().expect("cached snapshot");

        document.set_cell(0, 0, "b");
        let rebuilt = document.write_snapshot().expect("rebuilt snapshot");

        assert!(!Arc::ptr_eq(&cached, &rebuilt));
        assert_eq!(rebuilt.table.cell(0, 0), "b");
    }

    #[test]
    fn a_change_that_does_not_bump_the_revision_keeps_the_cache() {
        let mut document = document();
        let cached = document.write_snapshot().expect("cached snapshot");

        document.set_cell(0, 0, "a");

        let reused = document.write_snapshot().expect("reused snapshot");
        assert!(Arc::ptr_eq(&cached, &reused));
    }
}
