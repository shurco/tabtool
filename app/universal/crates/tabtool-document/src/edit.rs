//! Cell edits, insertion, deletion and one-step moves.
//!
//! Every mutation clones the shared table on write with [`Arc::make_mut`] and
//! compares the result with the previous content, so a command that changes
//! nothing records no undo step and does not bump the revision. Structural
//! actions clear the active sort, except column moves, which keep it.

use std::collections::BTreeSet;
use std::sync::Arc;

use crate::Document;
use crate::types::{DocumentPhase, GridSelection};
use crate::undo::UndoAction;

mod publish;
mod structure;

impl Document {
    /// Replaces one cell with a targeted undo record.
    ///
    /// A large document routes to [`Document::set_large_cell`], which edits the
    /// overlay and reports refusals through [`Document::notice`]. Invalid
    /// indexes, an unchanged value and an in-memory document without a table
    /// are ignored, so a no-op edit records no undo step.
    pub fn set_cell(&mut self, row: usize, column: usize, text: &str) {
        if self.is_large_mode() {
            let _ = self.set_large_cell(text, row, column);
            return;
        }
        if !self.can_edit_content() {
            return;
        }
        let Some(old) = self
            .table
            .as_ref()
            .and_then(|table| table.columns.get(column))
            .and_then(|column| column.cells.get(row))
            .cloned()
        else {
            return;
        };
        if old == text {
            return;
        }
        if self.replace_cell(row, column, text) {
            self.record_cell_change(UndoAction::EditCell, row, column, old, text.to_owned());
        }
    }

    /// Inserts a blank row above the first selected row, or at the top.
    ///
    /// The new row becomes the whole selection.
    pub fn add_row_above_selection(&mut self) {
        if !self.can_edit_content() || !self.can_edit_row_structure() {
            return;
        }
        let row_count = self.table.as_ref().map_or(0, |table| table.row_count());
        let index = self
            .selection
            .rows
            .first()
            .copied()
            .unwrap_or(0)
            .min(row_count);
        if self.replace_table(UndoAction::AddRow, |table| table.add_row(index), None) {
            self.selection = GridSelection {
                rows: BTreeSet::from([index]),
                columns: BTreeSet::new(),
            };
        }
    }

    /// Inserts a blank row below the last selected row, or at the end.
    ///
    /// The new row becomes the whole selection.
    pub fn add_row_below_selection(&mut self) {
        if !self.can_edit_content() || !self.can_edit_row_structure() {
            return;
        }
        let row_count = self.table.as_ref().map_or(0, |table| table.row_count());
        let index = self
            .selection
            .rows
            .last()
            .map_or(row_count, |row| row.saturating_add(1))
            .min(row_count);
        if self.replace_table(UndoAction::AddRow, |table| table.add_row(index), None) {
            self.selection = GridSelection {
                rows: BTreeSet::from([index]),
                columns: BTreeSet::new(),
            };
        }
    }

    /// Inserts a blank column left of the first selected column, or at the left.
    ///
    /// The new column becomes the whole selection.
    pub fn add_column_left_of_selection(&mut self) {
        if !self.can_edit_content() {
            return;
        }
        let column_count = self.table.as_ref().map_or(0, |table| table.columns.len());
        let index = self
            .selection
            .columns
            .first()
            .copied()
            .unwrap_or(0)
            .min(column_count);
        let header = self.empty_header();
        if self.replace_table(
            UndoAction::AddColumn,
            |table| table.add_column(index, header.as_deref()),
            None,
        ) {
            self.selection = GridSelection {
                rows: BTreeSet::new(),
                columns: BTreeSet::from([index]),
            };
        }
    }

    /// Inserts a blank column right of the last selected column, or at the
    /// right edge.
    ///
    /// The new column becomes the whole selection.
    pub fn add_column_right_of_selection(&mut self) {
        if !self.can_edit_content() {
            return;
        }
        let column_count = self.table.as_ref().map_or(0, |table| table.columns.len());
        let index = self
            .selection
            .columns
            .last()
            .map_or(column_count, |column| column.saturating_add(1))
            .min(column_count);
        let header = self.empty_header();
        if self.replace_table(
            UndoAction::AddColumn,
            |table| table.add_column(index, header.as_deref()),
            None,
        ) {
            self.selection = GridSelection {
                rows: BTreeSet::new(),
                columns: BTreeSet::from([index]),
            };
        }
    }

    /// Replaces one cell without recording undo; undo and redo use it.
    ///
    /// A cell edit has no old-table snapshot, so the table is mutated through
    /// [`Arc::make_mut`] directly. When the document owns the table the text is
    /// changed in place; only a caller holding a snapshot makes it copy.
    pub(crate) fn replace_cell(&mut self, row: usize, column: usize, text: &str) -> bool {
        let Some(table) = self.table.as_mut() else {
            return false;
        };
        {
            let Some(current) = table
                .columns
                .get(column)
                .and_then(|column| column.cells.get(row))
            else {
                return false;
            };
            if current == text {
                return false;
            }
        }
        // A save leaves the snapshot cache holding a second `Arc` of this
        // table. Drop that reference first, so `make_mut` mutates the owned
        // table instead of deep-copying it on the first edit after a save.
        self.snapshot_cache = None;
        Arc::make_mut(table).set_cell(text.to_owned(), row, column);
        self.bump_revision();
        self.refresh_phase();
        true
    }

    /// Whether the document has loaded content that can be edited.
    ///
    /// A re-read in flight owns the document, so edits are refused until it
    /// finishes instead of racing the replacement table.
    pub(crate) fn can_edit_content(&self) -> bool {
        !self.rereading
            && self.table.is_some()
            && matches!(
                self.phase,
                DocumentPhase::Ready | DocumentPhase::NeedsAttention(_)
            )
    }

    /// The header for a new column: empty in header mode, disabled otherwise.
    fn empty_header(&self) -> Option<String> {
        self.configuration.first_row_as_header.then(String::new)
    }
}

#[cfg(test)]
mod tests {
    use tabtool_core::{CsvConfiguration, SortDirection};

    use super::*;
    use crate::types::ActiveSort;

    fn document() -> Document {
        let configuration = CsvConfiguration::default();
        let mut document = Document::new(configuration.clone());
        document
            .load_from_bytes(b"b\na\nc\n", None, Some(configuration), false)
            .expect("load the fixture");
        document
    }

    fn sort() -> ActiveSort {
        ActiveSort {
            column_id: 1,
            direction: SortDirection::Ascending,
            source_order: vec![1, 0, 2],
        }
    }

    #[test]
    fn a_cell_edit_keeps_the_active_sort() {
        let mut document = document();
        document.active_sort = Some(sort());

        document.set_cell(0, 0, "z");

        assert_eq!(document.active_sort, Some(sort()));
    }

    #[test]
    fn an_unshared_cell_edit_reuses_the_table_allocation() {
        let mut document = document();
        let allocation = Arc::as_ptr(document.table().expect("table")) as usize;

        document.set_cell(0, 0, "z");

        assert_eq!(
            Arc::as_ptr(document.table().expect("table")) as usize,
            allocation
        );
    }

    #[test]
    fn structural_changes_clear_the_active_sort() {
        let mut document = document();
        document.active_sort = Some(sort());
        document.add_row_above_selection();
        assert_eq!(document.active_sort, None);

        document.active_sort = Some(sort());
        document.delete_rows(&BTreeSet::from([0]));
        assert_eq!(document.active_sort, None);

        document.active_sort = Some(sort());
        document.selection.rows.insert(0);
        document.move_selected_rows(1);
        assert_eq!(document.active_sort, None);
    }

    #[test]
    fn column_moves_keep_the_active_sort() {
        let mut document = document();
        document.active_sort = Some(sort());
        document.selection.columns.insert(0);

        document.move_selected_columns(1);
        assert_eq!(document.active_sort, Some(sort()));

        document.move_columns(&BTreeSet::from([1]), 0);
        assert_eq!(document.active_sort, Some(sort()));
    }
}
