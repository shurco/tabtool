//! Column-oriented table used by the document layer, grid, filters, sorting
//! and export.
//!
//! Every column always holds exactly [`CsvTable::row_count`] cells, the table
//! keeps at least one column, and headers are either present on every column
//! or absent from every column. Column ids are process-unique and only carry
//! identity: [`CsvTable`] equality compares content alone.

use std::collections::BTreeSet;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::reorder::is_permutation;
use crate::{column_name, is_numeric};

/// Process-unique identity of a column, stable for the session.
pub type ColumnId = u64;

/// Identity of a filter criterion, declared here for the filter layer.
pub type CriterionId = u64;

/// A set of row indexes.
pub type RowSet = BTreeSet<usize>;

static NEXT_COLUMN_ID: AtomicU64 = AtomicU64::new(1);

fn next_column_id() -> ColumnId {
    NEXT_COLUMN_ID.fetch_add(1, Ordering::Relaxed)
}

/// One column of a [`CsvTable`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CsvColumn {
    /// Session identity of the column.
    pub id: ColumnId,
    /// Column name, or `None` when the table has no headers.
    pub header: Option<String>,
    /// Cell text, one entry per row.
    pub cells: Vec<String>,
}

impl CsvColumn {
    /// Creates a column with a freshly allocated id.
    pub fn new(header: Option<String>, cells: Vec<String>) -> Self {
        Self {
            id: next_column_id(),
            header,
            cells,
        }
    }
}

/// A column-oriented CSV table.
///
/// Initializers pad ragged input with empty strings and never truncate it.
/// Headers are either present on every column or absent from every column.
#[derive(Clone, Debug)]
pub struct CsvTable {
    /// The columns, in visual order.
    pub columns: Vec<CsvColumn>,
}

impl CsvTable {
    /// Creates a table from columns, padding short columns with empty cells.
    ///
    /// A partially present set of headers is discarded as a whole so headers
    /// stay all-or-nothing. An empty input keeps one empty column.
    pub fn new(columns: Vec<CsvColumn>) -> Self {
        let mut columns = columns;
        if columns.is_empty() {
            columns.push(CsvColumn::new(None, Vec::new()));
        }

        let row_count = columns
            .iter()
            .map(|column| column.cells.len())
            .max()
            .unwrap_or(0);
        for column in &mut columns {
            column.cells.resize(row_count, String::new());
        }
        if columns.iter().any(|column| column.header.is_none()) {
            for column in &mut columns {
                column.header = None;
            }
        }

        Self { columns }
    }

    /// Creates a table from rows, padding every row to the widest row.
    ///
    /// `minimum_column_count` guarantees at least that many columns. Headers
    /// also reserve columns, and missing names are padded with empty strings so
    /// headers stay all-or-nothing.
    pub fn from_rows(
        rows: Vec<Vec<String>>,
        headers: Option<Vec<String>>,
        minimum_column_count: usize,
    ) -> Self {
        let row_width = rows.iter().map(Vec::len).max().unwrap_or(0);
        let header_width = headers.as_ref().map_or(0, Vec::len);
        let column_count = minimum_column_count.max(row_width).max(header_width);

        let mut columns = Vec::with_capacity(column_count);
        for column_index in 0..column_count {
            let cells = rows
                .iter()
                .map(|row| row.get(column_index).cloned().unwrap_or_default())
                .collect();
            let header = headers
                .as_ref()
                .map(|names| names.get(column_index).cloned().unwrap_or_default());
            columns.push(CsvColumn::new(header, cells));
        }

        Self::new(columns)
    }

    /// Creates a blank document with at least one row and one column.
    pub fn new_document(rows: usize, columns: usize) -> Self {
        Self::from_rows(vec![Vec::new(); rows.max(1)], None, columns.max(1))
    }

    /// The number of cells in each column.
    pub fn row_count(&self) -> usize {
        self.columns.first().map_or(0, |column| column.cells.len())
    }

    /// The headers of all columns, or `None` when the table has no headers.
    pub fn headers(&self) -> Option<Vec<String>> {
        if self.columns.is_empty() || self.columns.iter().any(|column| column.header.is_none()) {
            return None;
        }

        Some(
            self.columns
                .iter()
                .map(|column| column.header.clone().unwrap_or_default())
                .collect(),
        )
    }

    /// Replaces all headers, or removes them when `headers` is `None`.
    ///
    /// Missing names become empty strings; extra names are ignored.
    pub fn set_headers(&mut self, headers: Option<Vec<String>>) {
        match headers {
            None => {
                for column in &mut self.columns {
                    column.header = None;
                }
            }
            Some(names) => {
                for (index, column) in self.columns.iter_mut().enumerate() {
                    column.header = Some(names.get(index).cloned().unwrap_or_default());
                }
            }
        }
    }

    /// Returns the cell text, or an empty string when either index is invalid.
    pub fn cell(&self, row: usize, column: usize) -> String {
        self.columns
            .get(column)
            .and_then(|column| column.cells.get(row))
            .cloned()
            .unwrap_or_default()
    }

    /// Replaces a cell's text, ignoring invalid indexes.
    pub fn set_cell(&mut self, value: String, row: usize, column: usize) {
        if let Some(cell) = self
            .columns
            .get_mut(column)
            .and_then(|column| column.cells.get_mut(row))
        {
            *cell = value;
        }
    }

    /// Renames one column's header.
    ///
    /// The name may be empty. Without headers, every column first gets its
    /// generated name (A, B, C…) and all rows stay data rows. Returns `false`
    /// and changes nothing when `index` is invalid or a table without headers
    /// is "renamed" to the column's generated name.
    pub fn rename_header(&mut self, name: &str, index: usize) -> bool {
        if index >= self.columns.len() {
            return false;
        }
        if self.headers().is_none() {
            if name == column_name(index) {
                return false;
            }
            for (column_index, column) in self.columns.iter_mut().enumerate() {
                column.header = Some(column_name(column_index));
            }
        }
        self.columns[index].header = Some(name.to_owned());
        true
    }

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
        let new_header = if self.headers().is_some() {
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

    /// Converts the decimal mark in numeric cells from `from` to `to`.
    pub fn convert_decimal_mark(&mut self, from: &str, to: &str) {
        if from.is_empty() || from == to {
            return;
        }

        for column in &mut self.columns {
            for cell in &mut column.cells {
                if is_numeric(cell, from) {
                    *cell = cell.replace(from, to);
                }
            }
        }
    }

    /// Whether both tables hold the same column identities in the same order.
    ///
    /// Content equality alone cannot detect a column move when the moved
    /// columns contain identical values, so document mutations also compare
    /// this.
    pub fn has_same_column_order(&self, other: &CsvTable) -> bool {
        self.columns
            .iter()
            .map(|column| column.id)
            .eq(other.columns.iter().map(|column| column.id))
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

impl PartialEq for CsvTable {
    /// Compares document content, ignoring the session identity of columns.
    ///
    /// Two tables with the same headers and cell values are equal even when
    /// their columns carry different ids, as happens after re-reading a file.
    /// Use [`CsvTable::has_same_column_order`] when identity or visual order
    /// must also be checked.
    fn eq(&self, other: &Self) -> bool {
        self.headers() == other.headers()
            && self
                .columns
                .iter()
                .map(|column| &column.cells)
                .eq(other.columns.iter().map(|column| &column.cells))
    }
}

impl Eq for CsvTable {}
