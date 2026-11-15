//! Persistent grid state, header names and text alignment.

mod sync;

use std::collections::BTreeMap;

use egui::{Rect, Vec2};
use tabtool_core::{ColumnId, RowSource, column_name, is_numeric};
use tabtool_document::GridSelection as DocumentSelection;

use crate::grid::drag::{ColumnDrag, RowDrag};
use crate::grid::editor::{CellEditor, HeaderEditor};
use crate::grid::selection::{GridAnchor, GridFocus, GridSelection};

/// The grid's persistent state.
///
/// It outlives the current [`RowSource`](tabtool_core::RowSource), so scroll,
/// selection, widths and the editor survive a source swap after indexing.
#[derive(Clone, Debug)]
pub struct GridState {
    /// The selected rows or columns.
    pub selection: GridSelection,
    /// The cell being edited, if any.
    pub editing: Option<CellEditor>,
    /// The column header being renamed, if any.
    pub header_editing: Option<HeaderEditor>,
    /// The column drag in flight, if any.
    pub column_drag: Option<ColumnDrag>,
    /// The row drag in flight, if any.
    pub row_drag: Option<RowDrag>,
    /// The width of every column, keyed by its stable identity so a column
    /// move does not shift another column's width.
    pub widths: BTreeMap<ColumnId, f32>,
    /// The scroll offset in points.
    pub scroll: Vec2,
    /// The last visible rect in content coordinates.
    pub viewport: Rect,
    /// A cell that must be scrolled into view on the next paint.
    ///
    /// Touch editing opens a native editor over a cell, so the cell must be
    /// above the keyboard before the editor is placed.
    pub pending_scroll: Option<GridFocus>,
    /// The last hovered cell.
    pub hovered: Option<(usize, usize)>,
    /// The focused cell for keyboard navigation and editing.
    pub focus: Option<GridFocus>,
    /// The row or column a Shift click extends from.
    pub anchor: Option<GridAnchor>,
    /// Whether the grid owned keyboard focus at the end of the last frame.
    pub(crate) grid_has_focus: bool,
    /// A focus target to keep requesting until the scrolled-to cell exists.
    pub(crate) pending_focus: Option<GridFocus>,
    /// Whether a grid context menu was open at the end of the last frame.
    pub(crate) menu_open: bool,
    /// Set when a newly started editor must take keyboard focus.
    pub(crate) pending_editor_focus: bool,
    /// Set when the header rename editor must take keyboard focus.
    pub(crate) pending_header_focus: bool,
    /// Whether an IME preedit is active in the cell editor.
    pub(crate) ime_composing: bool,
    /// One-shot guard so an IME commit's Enter never commits the cell.
    pub(crate) ime_commit_guard: bool,
}

impl Default for GridState {
    fn default() -> Self {
        Self {
            selection: GridSelection::default(),
            editing: None,
            header_editing: None,
            column_drag: None,
            row_drag: None,
            widths: BTreeMap::new(),
            scroll: Vec2::ZERO,
            viewport: Rect::ZERO,
            pending_scroll: None,
            hovered: None,
            focus: None,
            anchor: None,
            grid_has_focus: false,
            pending_focus: None,
            menu_open: false,
            pending_editor_focus: false,
            pending_header_focus: false,
            ime_composing: false,
            ime_commit_guard: false,
        }
    }
}

impl GridState {
    /// Replaces the selection with a document selection, dropping grid-only
    /// state (the Shift anchor and focus).
    pub fn apply_document_selection(&mut self, selection: &DocumentSelection) {
        self.selection = GridSelection::from(selection);
        self.anchor = None;
    }

    /// The selected rows or columns in the document's representation.
    pub fn document_selection(&self) -> DocumentSelection {
        DocumentSelection::from(&self.selection)
    }

    /// Gives the grid a cell focus and requests it on the next frame.
    ///
    /// Callers that remap a focus after a structural change use this so the
    /// keyboard follows the moved row.
    pub fn focus_cell(&mut self, focus: Option<GridFocus>) {
        self.focus = focus;
        self.pending_focus = focus;
    }

    /// Scrolls `focus` into view on the next paint.
    ///
    /// Touch editing needs the cell above the keyboard before the platform
    /// editor is placed over it.
    pub fn scroll_to_cell(&mut self, focus: GridFocus) {
        self.pending_scroll = Some(focus);
    }

    /// Starts editing a cell with its current text and the caret at the end.
    pub fn start_editing(&mut self, row: usize, column: usize, text: impl Into<String>) {
        self.selection
            .select_row(row, false, false, &mut self.anchor);
        self.focus = Some(GridFocus::new(row, column));
        self.editing = Some(CellEditor::new(row, column, text));
        self.pending_editor_focus = true;
        self.ime_composing = false;
        self.ime_commit_guard = false;
        self.header_editing = None;
        self.pending_header_focus = false;
    }

    /// Starts renaming `column` with its current header text.
    ///
    /// The editor is keyed by the column's identity, so a reorder during the
    /// edit still renames the same column. Returns `false` when the column does
    /// not exist.
    pub fn start_renaming(&mut self, source: &dyn RowSource, column: usize) -> bool {
        let Some(id) = source.column_ids().get(column).copied() else {
            return false;
        };
        self.header_editing = Some(HeaderEditor {
            column: id,
            text: header_for(source.headers().as_deref(), column),
        });
        self.pending_header_focus = true;
        self.ime_composing = false;
        self.ime_commit_guard = false;
        self.editing = None;
        self.pending_editor_focus = false;
        true
    }
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
