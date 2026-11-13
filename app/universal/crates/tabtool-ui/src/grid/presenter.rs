//! The filtered, read-only projection of a [`RowSource`].

use std::collections::BTreeSet;

use tabtool_core::{ColumnId, RowSource};

use super::state::{CellAlignment, header_for};

/// A filtered, read-only projection of a [`RowSource`].
///
/// With `visible_rows` the grid shows only those rows; row arguments are then
/// display indexes while the source keeps model indexes. Row numbers in the
/// grid are display numbers (`1..visible row count`), not model numbers. The
/// presenter also implements [`RowSource`], so a filtered view can be handed to
/// the grid directly.
#[derive(Clone, Copy)]
pub struct GridPresenter<'a> {
    /// The underlying rows.
    pub source: &'a dyn RowSource,
    /// Ascending model indexes of the displayed rows, or `None` for all rows.
    pub visible_rows: Option<&'a [usize]>,
}

impl<'a> GridPresenter<'a> {
    /// Projects `source`, optionally through `visible_rows`.
    pub fn new(source: &'a dyn RowSource, visible_rows: Option<&'a [usize]>) -> Self {
        Self {
            source,
            visible_rows,
        }
    }

    /// The number of displayed rows.
    pub fn row_count(&self) -> usize {
        self.visible_rows
            .map_or_else(|| self.source.row_count(), <[usize]>::len)
    }

    /// The number of columns.
    pub fn column_count(&self) -> usize {
        self.source.column_count()
    }

    /// The header of `index`, or a generated A, B, … name without headers.
    pub fn header(&self, index: usize) -> String {
        if index >= self.column_count() {
            return String::new();
        }
        let headers = self.source.headers();
        header_for(headers.as_deref(), index)
    }

    /// The text of a display cell.
    pub fn cell(&self, row: usize, column: usize) -> String {
        self.model_row(row)
            .map(|model_row| self.source.cell(model_row, column))
            .unwrap_or_default()
    }

    /// Whether a display cell is numeric and therefore right-aligned.
    pub fn alignment(&self, row: usize, column: usize, decimal_mark: &str) -> CellAlignment {
        CellAlignment::of(&self.cell(row, column), decimal_mark)
    }

    /// The model row shown at display row `row`.
    pub fn model_row(&self, row: usize) -> Option<usize> {
        match self.visible_rows {
            Some(visible_rows) => visible_rows
                .get(row)
                .copied()
                .filter(|model_row| *model_row < self.source.row_count()),
            None if row < self.source.row_count() => Some(row),
            None => None,
        }
    }

    /// The display row of model row `row`, or `None` when it is hidden.
    pub fn display_row(&self, row: usize) -> Option<usize> {
        let Some(visible_rows) = self.visible_rows else {
            return (row < self.source.row_count()).then_some(row);
        };
        visible_rows.binary_search(&row).ok()
    }

    /// The model rows of `rows`, dropping hidden ones.
    pub fn model_rows(&self, rows: &BTreeSet<usize>) -> BTreeSet<usize> {
        rows.iter().filter_map(|row| self.model_row(*row)).collect()
    }

    /// The display rows of model `rows`, dropping hidden ones.
    pub fn display_rows(&self, rows: &BTreeSet<usize>) -> BTreeSet<usize> {
        rows.iter()
            .filter_map(|row| self.display_row(*row))
            .collect()
    }
}

impl RowSource for GridPresenter<'_> {
    fn row_count(&self) -> usize {
        GridPresenter::row_count(self)
    }

    fn column_count(&self) -> usize {
        GridPresenter::column_count(self)
    }

    fn column_ids(&self) -> Vec<ColumnId> {
        self.source.column_ids()
    }

    fn headers(&self) -> Option<Vec<String>> {
        self.source.headers()
    }

    fn cell(&self, row: usize, column: usize) -> String {
        GridPresenter::cell(self, row, column)
    }
}
