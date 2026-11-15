//! Column insertion, deletion and reordering on [`CsvTable`].

use super::{CsvColumn, CsvTable, RowSet};
use crate::reorder::is_permutation;

impl CsvTable {
    /// Reorders columns so result column `i` is current column `order[i]`.
    ///
    /// Input that is not a permutation of the column indexes is ignored.
    pub fn reorder_columns(&mut self, order: &[usize]) {
        if !is_permutation(order, self.columns.len()) {
            return;
        }

        let mut slots: Vec<Option<CsvColumn>> = std::mem::take(&mut self.columns)
            .into_iter()
            .map(Some)
            .collect();
        self.columns = order
            .iter()
            .map(|&index| slots[index].take().expect("order is a permutation"))
            .collect();
    }

    /// Inserts a blank column, clamping the index to the table bounds.
    ///
    /// `header` is used only when the table already has headers; otherwise the
    /// new column keeps headers disabled.
    pub fn add_column(&mut self, index: usize, header: Option<&str>) {
        let insertion = index.min(self.columns.len());
        let new_header = if self.has_headers() {
            Some(header.unwrap_or_default().to_owned())
        } else {
            None
        };
        let cells = vec![String::new(); self.row_count()];
        self.columns
            .insert(insertion, CsvColumn::new(new_header, cells));
    }

    /// Removes the columns at `indexes`; out-of-range indexes are ignored.
    ///
    /// Deleting every column leaves one empty column so a table always keeps at
    /// least one.
    pub fn delete_columns(&mut self, indexes: &RowSet) {
        let row_count = self.row_count();
        let selected: RowSet = indexes
            .iter()
            .copied()
            .filter(|&index| index < self.columns.len())
            .collect();
        for index in selected.iter().rev() {
            self.columns.remove(*index);
        }
        if self.columns.is_empty() {
            self.columns
                .push(CsvColumn::new(None, vec![String::new(); row_count]));
        }
    }

    /// Moves a column to a new position, clamping both indexes.
    pub fn move_column(&mut self, from: usize, to: usize) {
        if from >= self.columns.len() {
            return;
        }

        let target = to.min(self.columns.len() - 1);
        let column = self.columns.remove(from);
        self.columns.insert(target, column);
    }
}
