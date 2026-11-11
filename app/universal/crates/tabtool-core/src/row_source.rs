//! Read-only tabular view used by the grid, filters and sorting.

use crate::{ColumnId, CsvTable};

/// A read-only view of tabular data that a grid can display.
///
/// [`CsvTable`] implements it for in-memory documents. A large-file source can
/// implement it later and materialize only the rows being read.
pub trait RowSource {
    /// The number of cells in each column.
    fn row_count(&self) -> usize;

    /// The number of columns.
    fn column_count(&self) -> usize;

    /// The identities of the columns, in visual order.
    fn column_ids(&self) -> Vec<ColumnId>;

    /// The column names, or `None` when the data has no header row.
    fn headers(&self) -> Option<Vec<String>>;

    /// Returns the cell text, or an empty string for any invalid index.
    fn cell(&self, row: usize, column: usize) -> String;
}

impl RowSource for CsvTable {
    fn row_count(&self) -> usize {
        CsvTable::row_count(self)
    }

    fn column_count(&self) -> usize {
        self.columns.len()
    }

    fn column_ids(&self) -> Vec<ColumnId> {
        self.columns.iter().map(|column| column.id).collect()
    }

    fn headers(&self) -> Option<Vec<String>> {
        CsvTable::headers(self)
    }

    fn cell(&self, row: usize, column: usize) -> String {
        CsvTable::cell(self, row, column)
    }
}
