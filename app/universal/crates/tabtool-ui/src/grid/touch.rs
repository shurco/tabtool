//! Touch gesture rules and highlighting for the grid.
//!
//! The rules are pure so the first-tap/second-tap, long-press and
//! focus-beats-selection behavior is testable without a window. The render
//! pass feeds the same rules with the pointer results it receives.

use super::selection::{GridElement, GridFocus, GridSelection};

/// What a touch gesture asks the grid to do.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TouchGesture {
    /// Select the tapped cell's row.
    SelectCell,
    /// Open the platform editor for the already-focused cell.
    EditCell,
    /// Select the tapped column.
    SelectColumn,
    /// Select the tapped row from the row-number gutter.
    SelectRowNumber,
    /// Open the context menu for the pressed element.
    ContextMenu,
    /// Do nothing.
    Ignore,
}

/// The visual state of a cell under touch selection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GridHighlight {
    /// The cell is the tap target; it wins over its row or column fill.
    Focused,
    /// The cell belongs to a selected row or column.
    Selected,
    /// The cell has no selection.
    None,
}

/// An edit surface the touch grid asks its owner to open.
///
/// The grid never starts an in-window text field when [`GridOptions::touch`]
/// is set; the owner opens the platform editor over the requested cell or
/// column header instead.
///
/// [`GridOptions::touch`]: super::GridOptions::touch
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GridEditRequest {
    /// Edit a data cell, using its display row.
    Cell(GridFocus),
    /// Rename the column at this visual index.
    Column(usize),
}

/// The touch rule set.
///
/// The struct carries no state: the grid's persistent [`GridFocus`] and
/// [`GridSelection`] are the only inputs, so a re-render, a resize or a fold
/// cannot change what a tap means.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct GridTouch;

impl GridTouch {
    /// A stateless touch rule set.
    pub const fn new() -> Self {
        Self
    }

    /// What a tap on `element` does.
    ///
    /// A data cell is selected on the first tap; a tap on the cell that already
    /// has focus opens the platform editor. Headers select their whole axis and
    /// the corner does nothing.
    pub fn tap(
        &self,
        element: GridElement,
        focus: Option<GridFocus>,
        tapped: Option<GridFocus>,
    ) -> TouchGesture {
        match element {
            GridElement::ColumnHeader => TouchGesture::SelectColumn,
            GridElement::RowNumber => TouchGesture::SelectRowNumber,
            GridElement::Cell => match tapped {
                Some(tapped) if focus == Some(tapped) => TouchGesture::EditCell,
                Some(_) => TouchGesture::SelectCell,
                None => TouchGesture::Ignore,
            },
            GridElement::Corner => TouchGesture::Ignore,
        }
    }

    /// What a long press does: every grid element except the corner offers a
    /// context menu.
    pub fn long_press(&self, element: GridElement) -> TouchGesture {
        match element {
            GridElement::Corner => TouchGesture::Ignore,
            GridElement::Cell | GridElement::ColumnHeader | GridElement::RowNumber => {
                TouchGesture::ContextMenu
            }
        }
    }

    /// A double tap is reserved for zoom and never edits or selects.
    ///
    /// Android does not zoom the grid today; the rule keeps the reserved
    /// gesture from reaching the cell editor by accident.
    pub fn double_tap(&self) -> TouchGesture {
        TouchGesture::Ignore
    }

    /// Whether `cell` is focused, selected or plain.
    pub fn highlight(
        &self,
        selection: &GridSelection,
        focus: Option<GridFocus>,
        row: usize,
        column: usize,
    ) -> GridHighlight {
        if focus == Some(GridFocus::new(row, column)) {
            return GridHighlight::Focused;
        }
        if selection.rows.contains(&row) || selection.columns.contains(&column) {
            return GridHighlight::Selected;
        }
        GridHighlight::None
    }
}
