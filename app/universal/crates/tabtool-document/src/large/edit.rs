//! Overlay cell edits, header toggles and revert of a large document.

use tabtool_core::CsvError;

use crate::Document;
use crate::types::{DocumentNotice, LargeStatus};

impl Document {
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

    /// Records a refused edit as a one-shot notice and returns the error.
    fn reject_large_edit(&self, error: CsvError) -> Result<(), CsvError> {
        self.set_notice(DocumentNotice::EditRejected(error));
        Err(error)
    }
}
