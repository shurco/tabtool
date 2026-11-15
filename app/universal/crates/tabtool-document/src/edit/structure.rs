//! Deleting and moving rows and columns of the in-memory table.

use std::collections::BTreeSet;

use tabtool_core::shifted;

use crate::Document;
use crate::types::GridSelection;
use crate::undo::UndoAction;

impl Document {
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
