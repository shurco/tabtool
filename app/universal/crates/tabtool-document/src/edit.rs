//! Cell edits, insertion, deletion and one-step moves.
//!
//! Every mutation clones the shared table on write with [`Arc::make_mut`] and
//! compares the result with the previous content, so a command that changes
//! nothing records no undo step and does not bump the revision. Structural
//! actions clear the active sort, except column moves, which keep it.

use std::collections::BTreeSet;
use std::sync::Arc;

use tabtool_core::{CsvConfiguration, CsvTable, shifted};

use crate::Document;
use crate::types::{ActiveSort, DocumentPhase, GridSelection};
use crate::undo::UndoAction;

impl Document {
    /// Replaces one cell with a targeted undo record.
    ///
    /// Invalid indexes, an unchanged value and a document without a table are
    /// ignored, so a no-op edit records no undo step.
    pub fn set_cell(&mut self, row: usize, column: usize, text: &str) {
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

    /// Deletes the selected rows, or the selected columns when no row is
    /// selected, and clears the selection.
    ///
    /// Rows win over columns when both axes carry a selection, matching the
    /// grid's Delete command.
    pub fn delete_selection(&mut self) {
        if !self.can_edit_content() {
            return;
        }
        let Some(table) = self.table.as_ref() else {
            return;
        };
        let row_count = table.row_count();
        let column_count = table.columns.len();

        if !self.selection.rows.is_empty() {
            if !self.can_edit_row_structure() {
                return;
            }
            let rows: BTreeSet<usize> = self
                .selection
                .rows
                .iter()
                .copied()
                .filter(|&row| row < row_count)
                .collect();
            if rows.is_empty() {
                return;
            }
            let revision = self.revision;
            self.delete_rows(&rows);
            if self.revision != revision {
                self.selection = GridSelection::default();
            }
            return;
        }

        if self.selection.columns.is_empty() {
            return;
        }
        let columns: BTreeSet<usize> = self
            .selection
            .columns
            .iter()
            .copied()
            .filter(|&column| column < column_count)
            .collect();
        if columns.is_empty() {
            return;
        }
        let revision = self.revision;
        self.delete_columns(&columns);
        if self.revision != revision {
            self.selection = GridSelection::default();
        }
    }

    /// Removes the rows at `rows`; invalid indexes are ignored.
    ///
    /// The active sort is cleared and the selection is trimmed to the remaining
    /// rows.
    pub fn delete_rows(&mut self, rows: &BTreeSet<usize>) {
        if !self.can_edit_content() || !self.can_edit_row_structure() {
            return;
        }
        self.replace_table(
            UndoAction::DeleteRows,
            |table| table.delete_rows(rows),
            None,
        );
    }

    /// Removes the columns at `columns`; invalid indexes are ignored.
    ///
    /// Selecting every column keeps the leftmost selected column, so the table
    /// never loses its last column. The active sort is cleared and the
    /// selection is trimmed.
    pub fn delete_columns(&mut self, columns: &BTreeSet<usize>) {
        if !self.can_edit_content() {
            return;
        }
        let column_count = self.table.as_ref().map_or(0, |table| table.columns.len());
        let selected: BTreeSet<usize> = columns
            .iter()
            .copied()
            .filter(|&column| column < column_count)
            .collect();
        if selected.is_empty() {
            return;
        }
        let to_delete = if selected.len() == column_count {
            selected.iter().skip(1).copied().collect()
        } else {
            selected
        };
        if to_delete.is_empty() {
            return;
        }
        self.replace_table(
            UndoAction::DeleteColumns,
            |table| table.delete_columns(&to_delete),
            None,
        );
    }

    /// Moves the rows at `rows` before `destination`, clamped to the bounds.
    ///
    /// The active sort is cleared; the selection is trimmed but does not follow
    /// the moved rows.
    pub fn move_rows(&mut self, rows: &BTreeSet<usize>, destination: usize) {
        if !self.can_edit_content() || !self.can_edit_row_structure() {
            return;
        }
        self.replace_table(
            UndoAction::MoveRows,
            |table| table.move_rows(rows, destination),
            None,
        );
    }

    /// Moves the columns at `columns` before `destination`, clamped to the
    /// bounds.
    ///
    /// Column moves keep the active sort, because row order and column
    /// identities are unchanged.
    pub fn move_columns(&mut self, columns: &BTreeSet<usize>, destination: usize) {
        if !self.can_edit_content() {
            return;
        }
        let column_count = self.table.as_ref().map_or(0, |table| table.columns.len());
        let selected: BTreeSet<usize> = columns
            .iter()
            .copied()
            .filter(|&column| column < column_count)
            .collect();
        if selected.is_empty() {
            return;
        }
        let order = column_order(column_count, &selected, destination);
        let sort = self.active_sort.clone();
        self.replace_table(
            UndoAction::MoveColumn,
            |table| table.reorder_columns(&order),
            sort,
        );
    }

    /// Moves the selected rows one step by `by`; the selection follows them.
    ///
    /// A step that would push any selected row past an edge is ignored.
    pub fn move_selected_rows(&mut self, by: isize) {
        if !self.can_edit_content() || !self.can_edit_row_structure() {
            return;
        }
        let row_count = self.table.as_ref().map_or(0, |table| table.row_count());
        let selected: BTreeSet<usize> = self
            .selection
            .rows
            .iter()
            .copied()
            .filter(|&row| row < row_count)
            .collect();
        let Some(order) = shifted(row_count, &selected, by) else {
            return;
        };
        let selected_after = shifted_indexes(&selected, by);
        self.replace_table(
            UndoAction::MoveRows,
            |table| table.reorder_rows(&order),
            None,
        );
        self.selection = GridSelection {
            rows: selected_after,
            columns: BTreeSet::new(),
        };
    }

    /// Moves the selected columns one step by `by`; the selection follows them.
    ///
    /// A step that would push any selected column past an edge is ignored. The
    /// active sort is kept.
    pub fn move_selected_columns(&mut self, by: isize) {
        if !self.can_edit_content() {
            return;
        }
        let column_count = self.table.as_ref().map_or(0, |table| table.columns.len());
        let selected: BTreeSet<usize> = self
            .selection
            .columns
            .iter()
            .copied()
            .filter(|&column| column < column_count)
            .collect();
        let Some(order) = shifted(column_count, &selected, by) else {
            return;
        };
        let selected_after = shifted_indexes(&selected, by);
        let sort = self.active_sort.clone();
        self.replace_table(
            UndoAction::MoveColumn,
            |table| table.reorder_columns(&order),
            sort,
        );
        self.selection = GridSelection {
            rows: BTreeSet::new(),
            columns: selected_after,
        };
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

    /// Applies a table mutation and records the previous state for undo.
    ///
    /// Returns `false` without a revision, sort change or undo entry when the
    /// mutation leaves the content and column order unchanged.
    pub(crate) fn replace_table(
        &mut self,
        action: UndoAction,
        mutate: impl FnOnce(&mut CsvTable),
        next_sort: Option<ActiveSort>,
    ) -> bool {
        let Some(current) = self.table.as_ref().map(Arc::clone) else {
            return false;
        };
        let mut updated = Arc::clone(&current);
        mutate(Arc::make_mut(&mut updated));
        let configuration = self.configuration.clone();
        let includes_utf8_bom = self.had_utf8_bom;
        self.publish_content(action, updated, configuration, includes_utf8_bom, next_sort)
    }

    /// Applies a table prepared by the caller and records the previous state.
    ///
    /// Sorting and pasting build their table from a COW clone, so they hand
    /// the prepared value in instead of mutating a second copy.
    pub(crate) fn replace_table_with(
        &mut self,
        action: UndoAction,
        updated: Arc<CsvTable>,
        next_sort: Option<ActiveSort>,
    ) -> bool {
        if self.table.is_none() {
            return false;
        }
        let configuration = self.configuration.clone();
        let includes_utf8_bom = self.had_utf8_bom;
        self.publish_content(action, updated, configuration, includes_utf8_bom, next_sort)
    }

    /// Publishes prepared table, format and BOM state after the no-op check.
    ///
    /// This is the one path every structural or format action shares, so the
    /// revision, phase, filter refresh and undo snapshot cannot go stale.
    /// Equal content, column order, configuration and BOM record nothing,
    /// matching the Swift `applyChange` early return. A document without a
    /// table still accepts a first table, without an undo snapshot.
    pub(crate) fn publish_content(
        &mut self,
        action: UndoAction,
        updated: Arc<CsvTable>,
        configuration: CsvConfiguration,
        includes_utf8_bom: bool,
        next_sort: Option<ActiveSort>,
    ) -> bool {
        if let Some(current) = self.table.as_ref()
            && current.as_ref() == updated.as_ref()
            && current.has_same_column_order(updated.as_ref())
            && self.configuration == configuration
            && self.had_utf8_bom == includes_utf8_bom
        {
            return false;
        }

        let before = self.capture_state();
        self.table = Some(updated);
        self.configuration = configuration;
        self.had_utf8_bom = includes_utf8_bom;
        self.active_sort = next_sort;
        self.trim_selection();
        self.bump_revision();
        self.refresh_phase();
        self.refresh_filter();
        if let Some(before) = before {
            self.record_state_change(action, before);
        }
        true
    }

    /// Drops selected indexes that no longer exist after a structural change.
    pub(crate) fn trim_selection(&mut self) {
        let Some(table) = self.table.as_ref() else {
            self.selection = GridSelection::default();
            return;
        };
        let row_count = table.row_count();
        let column_count = table.columns.len();
        self.selection.rows.retain(|&row| row < row_count);
        self.selection
            .columns
            .retain(|&column| column < column_count);
    }
}

/// The column permutation that moves `selected` before `destination`.
///
/// Matches the row-move insertion rule: the destination is clamped, and
/// selected columns removed above it do not count toward the insertion point.
fn column_order(count: usize, selected: &BTreeSet<usize>, destination: usize) -> Vec<usize> {
    let destination = destination.min(count);
    let insertion = destination
        - selected
            .iter()
            .filter(|&&column| column < destination)
            .count();
    let mut remaining = Vec::with_capacity(count - selected.len());
    let mut moving = Vec::with_capacity(selected.len());
    for column in 0..count {
        if selected.contains(&column) {
            moving.push(column);
        } else {
            remaining.push(column);
        }
    }
    let target = insertion.min(remaining.len());
    remaining.splice(target..target, moving);
    remaining
}

/// Maps shifted indexes with the offset that [`shifted`] accepted.
fn shifted_indexes(selected: &BTreeSet<usize>, by: isize) -> BTreeSet<usize> {
    selected
        .iter()
        .filter_map(|&index| index.checked_add_signed(by))
        .collect()
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
