//! Column header renaming.
//!
//! Renaming delegates to [`tabtool_core::CsvTable::rename_header`] and is one
//! undoable [`UndoAction::RenameColumn`] step. Without a header, renaming
//! turns header mode on: the generated names (A, B, C…) become the header and
//! every row stays a data row. Rows and column identities stay as they are, so
//! the active sort is kept.

use std::sync::Arc;

use crate::Document;
use crate::undo::UndoAction;

impl Document {
    /// Renames the header of `column`; an empty name is allowed.
    ///
    /// A document without a header gets generated names for every column
    /// first, so the renamed table always has headers. Renaming to the same
    /// name, or to a generated name while header mode is off, records
    /// nothing.
    pub fn rename_column(&mut self, column: usize, name: &str) {
        if !self.can_rename_column(column) {
            return;
        }
        let Some(current) = self.table.as_ref().map(Arc::clone) else {
            return;
        };
        let mut renamed = Arc::clone(&current);
        if !Arc::make_mut(&mut renamed).rename_header(name, column) {
            return;
        }

        let mut configuration = self.configuration.clone();
        configuration.first_row_as_header = true;
        let includes_utf8_bom = self.had_utf8_bom;
        let sort = self.active_sort.clone();
        self.publish_content(
            UndoAction::RenameColumn,
            renamed,
            configuration,
            includes_utf8_bom,
            sort,
        );
    }

    /// Whether `column` exists and can be renamed now.
    pub fn can_rename_column(&self, column: usize) -> bool {
        self.can_edit_content()
            && self
                .table
                .as_ref()
                .is_some_and(|table| column < table.columns.len())
    }
}
