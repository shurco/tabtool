//! Parsing-format changes and transactional re-reads.
//!
//! A parsing change (encoding, separator, quote or escape) re-reads the file
//! through [`Storage`] and replaces the table, configuration and byte order
//! mark only when parsing succeeds and the document revision is unchanged.
//! Decimal-mark and header changes transform the in-memory table instead.
//! Every successful change records one undo snapshot of table, configuration,
//! BOM, sort and selection.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use tabtool_core::{CsvConfiguration, CsvTable};

use crate::Document;
use crate::io;
use crate::storage::Storage;
use crate::types::DocumentError;
use crate::undo::UndoAction;

/// A table parsed from the file, ready to replace the document.
///
/// `expected_revision` is the revision captured before the parse. The
/// candidate applies only while the document still has that revision, so an
/// edit, format change, undo or Save As that lands during the parse wins.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParsedTable {
    /// The parsed table.
    pub table: CsvTable,
    /// The configuration the bytes were parsed with.
    pub configuration: CsvConfiguration,
    /// Whether the parsed bytes began with a UTF-8 byte order mark.
    pub had_utf8_bom: bool,
    /// The document revision the candidate was parsed against.
    pub expected_revision: u64,
}

impl Document {
    /// Applies a parsing configuration from the format bar.
    ///
    /// A file-backed document re-reads the file through `storage` and applies
    /// the parsed bytes only when the parse succeeds and nothing changed the
    /// document meanwhile. Unsaved changes require `force`; without it the
    /// call reports [`DocumentError::RequiresConfirmation`] and leaves the
    /// document untouched.
    ///
    /// An untitled document has nothing to re-read, so only its configuration
    /// changes. A failed read or parse keeps the previous table, configuration
    /// and undo history. A streaming large save in flight reports
    /// [`DocumentError::Busy`].
    pub fn apply_parsing_configuration(
        &mut self,
        new: CsvConfiguration,
        force: bool,
        storage: &dyn Storage,
    ) -> Result<(), DocumentError> {
        self.ensure_not_saving()?;
        if matches!(self.phase, crate::DocumentPhase::Loading) {
            return Err(DocumentError::NotReady);
        }

        // A large document re-reads its mapping with the new format; it never
        // materializes a table.
        if self.is_large_mode() {
            return self.reopen_large(new, force);
        }

        if self.file_path().is_none() {
            let Some(table) = self.table.as_ref().map(Arc::clone) else {
                return Err(DocumentError::NotReady);
            };
            let includes_utf8_bom = self.had_utf8_bom;
            self.apply_content_change(table, new, includes_utf8_bom, UndoAction::ChangeFormat);
            return Ok(());
        }

        if !self.begin_reread() {
            return Err(DocumentError::RequiresConfirmation);
        }
        let result = self.reparse_file(new, force, storage);
        self.end_reread();
        result
    }

    /// Applies a re-parsed candidate only when its revision is still current.
    ///
    /// A stale candidate is ignored, so an asynchronous caller cannot replace
    /// newer edits. A candidate parsed from disk makes the document clean.
    pub fn apply_reparsed_candidate(&mut self, candidate: ParsedTable) {
        let _ = self.apply_candidate(candidate, UndoAction::ChangeFormat);
    }

    /// Runs the format detector over the current bytes of the file on disk.
    ///
    /// A large document sniffs the head of its mapped file instead of reading
    /// the whole file into memory. The configuration is returned without
    /// changing the document. An untitled document has no bytes to read and
    /// returns its configuration.
    pub fn detect_format_from_disk(
        &self,
        storage: &dyn Storage,
        prefers_chinese: bool,
    ) -> Result<CsvConfiguration, DocumentError> {
        let Some(path) = self.file_path() else {
            return Ok(self.configuration.clone());
        };
        if self.is_large_mode() {
            let file =
                tabtool_large::MappedFile::open(path).map_err(|_| DocumentError::CannotReadFile)?;
            return tabtool_large::sniff(&file, prefers_chinese).map_err(DocumentError::Csv);
        }
        let bytes = storage
            .read(path)
            .map_err(|_| DocumentError::CannotReadFile)?;
        Ok(io::detect(&bytes, prefers_chinese))
    }

    /// Converts numeric cells from the current decimal mark to `mark`.
    ///
    /// Only numeric text changes; formatting such as leading zeros and
    /// exponents is preserved. The configuration follows the table so saving
    /// and pasting use the new mark. One undo snapshot covers both.
    pub fn set_decimal_mark(&mut self, mark: &str) {
        if !self.can_edit_content() || mark.is_empty() || mark == self.configuration.decimal_mark {
            return;
        }
        let Some(current) = self.table.as_ref().map(Arc::clone) else {
            return;
        };
        let from = self.configuration.decimal_mark.clone();
        let mut updated = Arc::clone(&current);
        Arc::make_mut(&mut updated).convert_decimal_mark(&from, mark);

        let mut configuration = self.configuration.clone();
        configuration.decimal_mark = mark.to_owned();
        let includes_utf8_bom = self.had_utf8_bom;
        self.apply_content_change(
            updated,
            configuration,
            includes_utf8_bom,
            UndoAction::ChangeFormat,
        );
    }

    /// Turns header mode on or off by moving the first row across the header
    /// boundary, updating both the table and the configuration.
    ///
    /// A large document routes to [`Document::set_large_header_enabled`], which
    /// moves the source's row mapping instead of a table. One undo snapshot
    /// restores the table, configuration and first row, or the large row
    /// mapping and configuration.
    pub fn set_header_enabled(&mut self, enabled: bool) {
        if self.is_large_mode() {
            self.set_large_header_enabled(enabled);
            return;
        }
        if !self.can_edit_content() {
            return;
        }
        let Some(current) = self.table.as_ref().map(Arc::clone) else {
            return;
        };
        let has_headers = current.headers().is_some();

        if has_headers == enabled {
            if self.configuration.first_row_as_header == enabled {
                return;
            }
            let mut configuration = self.configuration.clone();
            configuration.first_row_as_header = enabled;
            let includes_utf8_bom = self.had_utf8_bom;
            self.apply_content_change(
                current,
                configuration,
                includes_utf8_bom,
                UndoAction::ToggleHeader,
            );
            return;
        }

        let mut updated = Arc::clone(&current);
        if enabled {
            let headers: Vec<String> = (0..current.columns.len())
                .map(|column| current.cell(0, column))
                .collect();
            let table = Arc::make_mut(&mut updated);
            table.set_headers(Some(headers));
            table.delete_rows(&BTreeSet::from([0]));
        } else if let Some(headers) = current.headers() {
            let table = Arc::make_mut(&mut updated);
            table.set_headers(None);
            table.add_row(0);
            for (column, header) in headers.iter().enumerate() {
                table.set_cell(header.clone(), 0, column);
            }
        }

        let mut configuration = self.configuration.clone();
        configuration.first_row_as_header = enabled;
        let includes_utf8_bom = self.had_utf8_bom;
        self.apply_content_change(
            updated,
            configuration,
            includes_utf8_bom,
            UndoAction::ToggleHeader,
        );
    }

    /// Whether the content differs from the saved content.
    ///
    /// An untitled document always has unsaved changes: there is no file that
    /// could hold its content.
    pub fn has_unsaved_changes(&self) -> bool {
        self.is_dirty()
    }

    /// Marks a re-read in flight and returns `false` when one is already
    /// running. Only one re-read may replace the document at a time.
    pub fn begin_reread(&mut self) -> bool {
        if self.rereading {
            return false;
        }
        self.rereading = true;
        true
    }

    /// Clears the in-flight re-read marker.
    pub fn end_reread(&mut self) {
        self.rereading = false;
    }

    /// Reads and parses the file, then applies the candidate transactionally.
    pub(crate) fn read_and_apply(
        &mut self,
        path: &Path,
        configuration: CsvConfiguration,
        storage: &dyn Storage,
        action: UndoAction,
    ) -> Result<(), DocumentError> {
        let expected_revision = self.revision;
        let bytes = storage
            .read(path)
            .map_err(|_| DocumentError::CannotReadFile)?;
        let outcome = io::parse(&bytes, &configuration).map_err(DocumentError::Csv)?;
        let candidate = ParsedTable {
            table: outcome.table,
            configuration,
            had_utf8_bom: outcome.had_utf8_bom,
            expected_revision,
        };
        if self.apply_candidate(candidate, action) {
            Ok(())
        } else {
            Err(DocumentError::RequiresConfirmation)
        }
    }

    /// Re-reads a file-backed document after the dirty gate.
    fn reparse_file(
        &mut self,
        new: CsvConfiguration,
        force: bool,
        storage: &dyn Storage,
    ) -> Result<(), DocumentError> {
        if !force && self.table.is_some() && self.has_unsaved_changes() {
            return Err(DocumentError::RequiresConfirmation);
        }
        let Some(path) = self.file_path().map(PathBuf::from) else {
            return Err(DocumentError::NotReady);
        };
        self.read_and_apply(&path, new, storage, UndoAction::ChangeFormat)
    }

    /// Applies a candidate guarded by its captured revision.
    ///
    /// Returns `false` without changing anything when the candidate is stale.
    /// A successful apply makes the content match disk, so it is not dirty.
    fn apply_candidate(&mut self, candidate: ParsedTable, action: UndoAction) -> bool {
        if self.revision != candidate.expected_revision
            || self.large.is_some()
            || self.large_save.is_some()
            || matches!(self.large_status, crate::LargeStatus::Saving(_))
            || matches!(self.phase, crate::DocumentPhase::Loading)
        {
            return false;
        }
        self.apply_content_change(
            Arc::new(candidate.table),
            candidate.configuration,
            candidate.had_utf8_bom,
            action,
        );
        self.saved_revision = self.revision;
        true
    }

    /// Replaces table, configuration and BOM as one undoable change.
    ///
    /// The active sort is cleared, the selection is trimmed to the new bounds
    /// and the applied filter is refreshed. Content, column order,
    /// configuration and BOM that are all unchanged record nothing.
    fn apply_content_change(
        &mut self,
        table: Arc<CsvTable>,
        configuration: CsvConfiguration,
        includes_utf8_bom: bool,
        action: UndoAction,
    ) {
        self.publish_content(action, table, configuration, includes_utf8_bom, None);
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use tabtool_core::SortDirection;

    use super::*;
    use crate::types::ActiveSort;

    #[test]
    fn a_candidate_clears_the_sort_and_undo_restores_sort_and_selection() {
        let configuration = CsvConfiguration::default();
        let mut document = Document::new(configuration.clone());
        document
            .load_from_bytes(b"b\na\n", None, Some(configuration.clone()), false)
            .expect("load the fixture");
        document.active_sort = Some(ActiveSort {
            column_id: 1,
            direction: SortDirection::Ascending,
            source_order: vec![1, 0],
        });
        document.selection.rows.insert(1);

        document.apply_reparsed_candidate(ParsedTable {
            table: CsvTable::from_rows(vec![vec!["x".to_string()]], None, 1),
            configuration,
            had_utf8_bom: false,
            expected_revision: document.revision,
        });

        assert_eq!(document.active_sort, None);
        assert!(document.selection.rows.is_empty());

        assert!(document.undo());
        assert!(document.active_sort.is_some());
        assert_eq!(document.selection.rows, BTreeSet::from([1]));
    }
}
