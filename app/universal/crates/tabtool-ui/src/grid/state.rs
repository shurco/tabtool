//! Persistent grid state, display options and the row-source presenter.

use std::collections::BTreeSet;

use egui::Vec2;
use tabtool_core::{ColumnId, RowSource, SortDirection, column_name, is_numeric};

use crate::grid::editor::CellEditor;
use crate::grid::selection::{GridFocus, GridSelection};

/// The sort the header marks with an arrow.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SortSpec {
    /// The sorted visual column.
    pub column: usize,
    /// The sort direction.
    pub direction: SortDirection,
}

/// The grid presentation settings.
#[derive(Clone, Debug, PartialEq)]
pub struct GridOptions {
    /// The height of every data row.
    pub row_height: f32,
    /// Whether odd rows use a tinted background.
    pub alternating_rows: bool,
    /// Whether the row-number gutter is visible.
    pub shows_row_numbers: bool,
    /// Physical rows hidden above display row zero (large-mode headers).
    pub header_offset: usize,
    /// Whether cells can enter the editor.
    pub editable: bool,
    /// The document's decimal mark, used for numeric right alignment.
    pub decimal_mark: String,
    /// The width assigned to a column that has never been resized.
    pub default_column_width: f32,
    /// The narrowest a column may be resized to.
    pub min_column_width: f32,
    /// The content cap: the widest a column may be resized to.
    pub max_column_width: f32,
    /// The sort shown with an arrow in the header.
    pub sort: Option<SortSpec>,
}

impl Default for GridOptions {
    fn default() -> Self {
        Self {
            row_height: 24.0,
            alternating_rows: false,
            shows_row_numbers: true,
            header_offset: 0,
            editable: true,
            decimal_mark: ".".to_string(),
            default_column_width: crate::grid::geometry::DEFAULT_COLUMN_WIDTH,
            min_column_width: crate::grid::geometry::MIN_COLUMN_WIDTH,
            max_column_width: crate::grid::geometry::MAX_COLUMN_WIDTH,
            sort: None,
        }
    }
}

impl GridOptions {
    /// Whether the grid draws horizontal separators.
    ///
    /// Alternating backgrounds replace them, matching the macOS appearance.
    pub fn shows_horizontal_lines(&self) -> bool {
        !self.alternating_rows
    }

    /// The physical row a display row maps to.
    pub fn physical_row(&self, display_row: usize) -> usize {
        display_row.saturating_add(self.header_offset)
    }
}

/// The grid's persistent state.
///
/// It outlives the current [`RowSource`](tabtool_core::RowSource), so scroll,
/// selection, widths and the editor survive a source swap after indexing.
#[derive(Clone, Debug, Default)]
pub struct GridState {
    /// The selected rows or columns.
    pub selection: GridSelection,
    /// The cell being edited, if any.
    pub editing: Option<CellEditor>,
    /// The width of every column.
    pub widths: Vec<f32>,
    /// The scroll offset in points.
    pub scroll: Vec2,
    /// The last hovered cell.
    pub hovered: Option<(usize, usize)>,
    /// The focused cell for keyboard navigation and editing.
    pub focus: Option<GridFocus>,
    /// The row or column a Shift click extends from.
    pub anchor: Option<usize>,
    /// Set when a newly started editor must take keyboard focus.
    pub(crate) pending_editor_focus: bool,
}

impl GridState {
    /// Fits the state to the current source without discarding user state.
    ///
    /// Missing widths get the option default, extra widths are dropped, and
    /// selection, focus, hover and editing are clamped to the new bounds.
    pub fn sync(&mut self, source: &dyn RowSource, options: &GridOptions) {
        let rows = source.row_count();
        let columns = source.column_count();
        self.widths.resize(columns, options.default_column_width);
        for width in &mut self.widths {
            *width = clamp_width(*width, options);
        }
        self.selection.rows.retain(|row| *row < rows);
        self.selection.columns.retain(|column| *column < columns);
        if self.selection.rows.is_empty() && self.selection.columns.is_empty() {
            self.anchor = None;
        }
        if let Some(editing) = &self.editing
            && (editing.row >= rows || editing.column >= columns)
        {
            self.editing = None;
        }
        if self
            .focus
            .is_some_and(|focus| focus.row >= rows || focus.column >= columns)
        {
            self.focus = None;
        }
        if self
            .hovered
            .is_some_and(|(row, column)| row >= rows || column >= columns)
        {
            self.hovered = None;
        }
        if self
            .anchor
            .is_some_and(|anchor| anchor >= rows.max(columns))
        {
            self.anchor = None;
        }
    }

    /// Resizes one column by a pointer delta, clamped to the options.
    pub fn resize_column(&mut self, column: usize, delta: f32, options: &GridOptions) {
        if let Some(width) = self.widths.get_mut(column) {
            *width = clamp_width(*width + delta, options);
        }
    }

    /// Starts editing a cell with its current text and the caret at the end.
    pub fn start_editing(&mut self, row: usize, column: usize, text: impl Into<String>) {
        self.selection
            .select_row(row, false, false, &mut self.anchor);
        self.focus = Some(GridFocus::new(row, column));
        self.editing = Some(CellEditor::new(row, column, text));
        self.pending_editor_focus = true;
    }
}

fn clamp_width(width: f32, options: &GridOptions) -> f32 {
    let min = options.min_column_width.max(0.0);
    let max = options.max_column_width.max(min);
    width.clamp(min, max)
}

/// The header text for `column`, generated A, B, … when headers are absent.
pub(crate) fn header_for(headers: Option<&[String]>, column: usize) -> String {
    headers
        .and_then(|headers| headers.get(column).cloned())
        .unwrap_or_else(|| column_name(column))
}

/// The alignment of a cell's text.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CellAlignment {
    /// Text starts at the leading edge.
    Left,
    /// Numeric text ends at the trailing edge.
    Right,
}

impl CellAlignment {
    /// The alignment for cell text using the document's decimal mark.
    pub fn of(text: &str, decimal_mark: &str) -> Self {
        if is_numeric(text, decimal_mark) {
            Self::Right
        } else {
            Self::Left
        }
    }
}

/// A filtered, read-only projection of a [`RowSource`].
///
/// With `visible_rows` the grid shows only those rows; row arguments are then
/// display indexes while the source keeps model indexes. The presenter also
/// implements [`RowSource`], so a filtered view can be handed to the grid
/// directly.
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
