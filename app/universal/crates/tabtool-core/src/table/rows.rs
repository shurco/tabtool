//! Row insertion, deletion and reordering on [`CsvTable`].

use super::{CsvTable, RowSet};
use crate::reorder::is_permutation;

impl CsvTable {
    /// Inserts a blank row, clamping the index to the table bounds.
    pub fn add_row(&mut self, index: usize) {
        let insertion = index.min(self.row_count());
        for column in &mut self.columns {
            let safe_insertion = insertion.min(column.cells.len());
            column.cells.insert(safe_insertion, String::new());
        }
    }

    /// Removes the rows at `indexes`; out-of-range indexes are ignored.
    ///
    /// Every column is rebuilt in one pass. Removing selected indexes one by
    /// one costs O(rows × selected) and is too slow for a large selection on a
    /// 100k-row table.
    pub fn delete_rows(&mut self, indexes: &RowSet) {
        let row_count = self.row_count();
        let selected: RowSet = indexes
            .iter()
            .copied()
            .filter(|&row| row < row_count)
            .collect();
        if selected.is_empty() {
            return;
        }

        let order: Vec<usize> = (0..row_count)
            .filter(|row| !selected.contains(row))
            .collect();
        self.apply_row_order(&order);
    }

    /// Moves the rows at `indexes` so they end up before `destination`.
    ///
    /// `destination` is an index in the pre-move table, clamped to the table
    /// bounds. The insertion point is adjusted for selected rows removed above
    /// it, and moved rows keep their original order. Each column is rebuilt in
    /// one pass so a large move stays linear in the table size.
    pub fn move_rows(&mut self, indexes: &RowSet, destination: usize) {
        let row_count = self.row_count();
        let selected: RowSet = indexes
            .iter()
            .copied()
            .filter(|&row| row < row_count)
            .collect();
        if selected.is_empty() {
            return;
        }

        let destination = destination.min(row_count);
        let insertion = destination - selected.iter().filter(|&&row| row < destination).count();

        let mut remaining = Vec::with_capacity(row_count - selected.len());
        let mut moving = Vec::with_capacity(selected.len());
        for row in 0..row_count {
            if selected.contains(&row) {
                moving.push(row);
            } else {
                remaining.push(row);
            }
        }

        let target = insertion.min(remaining.len());
        remaining.splice(target..target, moving);
        self.apply_row_order(&remaining);
    }

    /// Reorders rows so result row `i` is current row `order[i]`.
    ///
    /// Input that is not a permutation of the row indexes is ignored.
    pub fn reorder_rows(&mut self, order: &[usize]) {
        if !is_permutation(order, self.row_count()) {
            return;
        }
        self.apply_row_order(order);
    }

    /// Rebuilds every column from a row-index order in one pass.
    fn apply_row_order(&mut self, order: &[usize]) {
        for column in &mut self.columns {
            let mut cells: Vec<Option<String>> = std::mem::take(&mut column.cells)
                .into_iter()
                .map(Some)
                .collect();
            column.cells = order
                .iter()
                .map(|&index| cells[index].take().expect("order is a permutation"))
                .collect();
        }
    }
}
