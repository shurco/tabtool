//! Persistent grid state, header names and text alignment.

use std::collections::BTreeMap;

use egui::{Rect, Vec2};
use tabtool_core::{ColumnId, RowSource, column_name, is_numeric};
use tabtool_document::GridSelection as DocumentSelection;

use crate::grid::drag::{ColumnDrag, RowDrag};
use crate::grid::editor::{CellEditor, HeaderEditor};
use crate::grid::geometry::DEFAULT_COLUMN_WIDTH;
use crate::grid::options::GridOptions;
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
    /// Fits the state to the current source without discarding user state.
    ///
    /// Missing widths get the option default, extra widths are dropped, and
    /// selection, focus, hover and editing are clamped to the new bounds.
    pub fn sync(&mut self, source: &dyn RowSource, options: &GridOptions) {
        let rows = source.row_count();
        let columns = source.column_count();
        let column_ids = source.column_ids();
        if self
            .header_editing
            .as_ref()
            .is_some_and(|editing| !column_ids.contains(&editing.column))
        {
            self.header_editing = None;
        }
        if self
            .column_drag
            .as_ref()
            .is_some_and(|drag| !column_ids.contains(&drag.column))
        {
            self.column_drag = None;
        }
        if let Some(drag) = &mut self.column_drag {
            drag.target = drag.target.min(columns);
        }
        if self
            .row_drag
            .as_ref()
            .is_some_and(|drag| drag.rows.iter().any(|row| *row >= rows))
        {
            self.row_drag = None;
        }
        if let Some(drag) = &mut self.row_drag
            && let Some(target) = drag.target
        {
            drag.target = Some(target.min(rows));
        }
        self.widths.retain(|id, _| column_ids.contains(id));
        for id in column_ids {
            let width = self
                .widths
                .entry(id)
                .or_insert(options.default_column_width);
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
            .pending_focus
            .is_some_and(|focus| focus.row >= rows || focus.column >= columns)
        {
            self.pending_focus = None;
        }
        if self
            .hovered
            .is_some_and(|(row, column)| row >= rows || column >= columns)
        {
            self.hovered = None;
        }
        self.anchor = match self.anchor {
            Some(GridAnchor::Row(row)) if row < rows => Some(GridAnchor::Row(row)),
            Some(GridAnchor::Column(column)) if column < columns => {
                Some(GridAnchor::Column(column))
            }
            _ => None,
        };
    }

    /// The column widths in `source`'s current visual order.
    pub fn ordered_widths(&self, source: &dyn RowSource) -> Vec<f32> {
        source
            .column_ids()
            .into_iter()
            .map(|id| {
                self.widths
                    .get(&id)
                    .copied()
                    .unwrap_or(DEFAULT_COLUMN_WIDTH)
            })
            .collect()
    }

    /// Resizes one column by a pointer delta, clamped to the options.
    pub fn resize_column(
        &mut self,
        source: &dyn RowSource,
        column: usize,
        delta: f32,
        options: &GridOptions,
    ) {
        let Some(id) = source.column_ids().get(column).copied() else {
            return;
        };
        let width = self
            .widths
            .entry(id)
            .or_insert(options.default_column_width);
        *width = clamp_width(*width + delta, options);
    }

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
