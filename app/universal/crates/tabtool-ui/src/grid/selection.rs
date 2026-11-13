//! Row/column selection and the pointer rules that resolve a tap into a
//! select, an edit or a column action.

use std::collections::BTreeSet;

use tabtool_document::GridSelection as DocumentSelection;

use crate::grid::keyboard::MoveDirection;

/// The row or column a Shift click extends from.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GridAnchor {
    /// A display row.
    Row(usize),
    /// A visual column.
    Column(usize),
}

/// The cell the keyboard and the editor act on. Rows are display rows.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct GridFocus {
    /// Display row index.
    pub row: usize,
    /// Visual column index.
    pub column: usize,
}

impl GridFocus {
    /// A focus at `row` and `column`.
    pub fn new(row: usize, column: usize) -> Self {
        Self { row, column }
    }
}

/// The rows or columns selected in the grid.
///
/// A selection is either a set of rows or a set of columns; the other axis is
/// cleared whenever the user changes it.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct GridSelection {
    /// Selected display rows.
    pub rows: BTreeSet<usize>,
    /// Selected visual columns.
    pub columns: BTreeSet<usize>,
}

impl GridSelection {
    /// An empty selection.
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether nothing is selected.
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty() && self.columns.is_empty()
    }

    /// Replaces or extends the row selection, clearing the column axis.
    ///
    /// A plain click selects one row, Shift extends from `anchor`, and
    /// Command toggles one row.
    pub fn select_row(
        &mut self,
        row: usize,
        extend: bool,
        toggle: bool,
        anchor: &mut Option<GridAnchor>,
    ) {
        self.columns.clear();
        if toggle {
            if !self.rows.remove(&row) {
                self.rows.insert(row);
                anchor.get_or_insert(GridAnchor::Row(row));
            }
            if self.rows.is_empty() {
                *anchor = None;
            }
        } else if extend {
            let base = match anchor {
                Some(GridAnchor::Row(base)) => Some(*base),
                _ => self.rows.first().copied(),
            }
            .unwrap_or(row);
            self.rows = (base.min(row)..=base.max(row)).collect();
        } else {
            self.rows.clear();
            self.rows.insert(row);
            *anchor = Some(GridAnchor::Row(row));
        }
    }

    /// Replaces or extends the column selection, clearing the row axis.
    ///
    /// A plain click selects one column, Shift extends from `anchor`, and
    /// Command toggles one column.
    pub fn select_column(
        &mut self,
        column: usize,
        extend: bool,
        toggle: bool,
        anchor: &mut Option<GridAnchor>,
    ) {
        self.rows.clear();
        if toggle {
            if !self.columns.remove(&column) {
                self.columns.insert(column);
                anchor.get_or_insert(GridAnchor::Column(column));
            }
            if self.columns.is_empty() {
                *anchor = None;
            }
        } else if extend {
            let base = match anchor {
                Some(GridAnchor::Column(base)) => Some(*base),
                _ => self.columns.first().copied(),
            }
            .unwrap_or(column);
            self.columns = (base.min(column)..=base.max(column)).collect();
        } else {
            self.columns.clear();
            self.columns.insert(column);
            *anchor = Some(GridAnchor::Column(column));
        }
    }
}

/// The grid element a tap landed on.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GridElement {
    /// A data cell.
    Cell,
    /// A pinned column header.
    ColumnHeader,
    /// The pinned row-number gutter.
    RowNumber,
    /// The pinned corner above the gutter.
    Corner,
}

/// What a tap should do, given the element and the focused cell.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GridTapOutcome {
    /// Select the tapped cell's row.
    SelectCell,
    /// Edit the already-focused cell.
    EditCell,
    /// Select the tapped column.
    SelectColumn,
    /// Select the tapped row from the gutter.
    SelectRowNumber,
    /// Do nothing.
    Ignore,
}

/// Decides the effect of a tap without a UI, so the rule is testable.
pub fn resolve_tap(
    element: GridElement,
    focus: Option<GridFocus>,
    tapped: Option<GridFocus>,
) -> GridTapOutcome {
    match element {
        GridElement::ColumnHeader => GridTapOutcome::SelectColumn,
        GridElement::RowNumber => GridTapOutcome::SelectRowNumber,
        GridElement::Cell => match tapped {
            Some(tapped) if focus == Some(tapped) => GridTapOutcome::EditCell,
            Some(_) => GridTapOutcome::SelectCell,
            None => GridTapOutcome::Ignore,
        },
        GridElement::Corner => GridTapOutcome::Ignore,
    }
}

/// Moves the focused cell with an arrow key, updating row selection.
///
/// Left and right keep the row selected; up and down move the row selection
/// (or extend it when `extend` is true). Returns the new focus, or `None` for
/// an empty grid.
pub fn move_focus(
    selection: &mut GridSelection,
    anchor: &mut Option<GridAnchor>,
    focus: &mut Option<GridFocus>,
    row_count: usize,
    column_count: usize,
    direction: MoveDirection,
    extend: bool,
) -> Option<GridFocus> {
    if row_count == 0 || column_count == 0 {
        return None;
    }
    let current =
        focus.unwrap_or_else(|| GridFocus::new(selection.rows.first().copied().unwrap_or(0), 0));
    let last_row = row_count - 1;
    let last_column = column_count - 1;
    let next = match direction {
        MoveDirection::Up => GridFocus::new(current.row.saturating_sub(1), current.column),
        MoveDirection::Down => GridFocus::new((current.row + 1).min(last_row), current.column),
        MoveDirection::Left => GridFocus::new(current.row, current.column.saturating_sub(1)),
        MoveDirection::Right => GridFocus::new(current.row, (current.column + 1).min(last_column)),
    };
    let next = GridFocus::new(next.row.min(last_row), next.column.min(last_column));
    match direction {
        MoveDirection::Up | MoveDirection::Down => {
            selection.select_row(next.row, extend, false, anchor);
        }
        MoveDirection::Left | MoveDirection::Right if selection.rows.is_empty() => {
            selection.select_row(next.row, false, false, anchor);
        }
        MoveDirection::Left | MoveDirection::Right => {}
    }
    *focus = Some(next);
    Some(next)
}

impl From<&DocumentSelection> for GridSelection {
    fn from(selection: &DocumentSelection) -> Self {
        Self {
            rows: selection.rows.clone(),
            columns: selection.columns.clone(),
        }
    }
}

impl From<&GridSelection> for DocumentSelection {
    fn from(selection: &GridSelection) -> Self {
        Self {
            rows: selection.rows.clone(),
            columns: selection.columns.clone(),
        }
    }
}

impl From<GridSelection> for DocumentSelection {
    fn from(selection: GridSelection) -> Self {
        Self {
            rows: selection.rows,
            columns: selection.columns,
        }
    }
}
