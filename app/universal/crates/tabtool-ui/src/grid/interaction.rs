//! Pointer gestures accumulated during the render pass and applied after it.
//!
//! Keeping the mutation here means painting and hit testing only read the grid
//! state; one place then updates selection, starts drags, opens the rename
//! editor and emits the actions a gesture requested.

mod apply;

use egui::Modifiers;

use super::context::MenuCommand;
use super::selection::{GridElement, GridFocus};
use super::touch::GridEditRequest;

/// A right-click that opens a context menu.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ContextTap {
    /// The element under the pointer.
    pub element: GridElement,
    /// The display row under the pointer, or zero for a header.
    pub row: usize,
    /// The visual column under the pointer.
    pub column: usize,
}

/// What the pointer requested this frame.
#[derive(Clone, Debug, Default)]
pub(crate) struct Interaction {
    cell: Option<(usize, usize, Modifiers)>,
    column: Option<(usize, Modifiers)>,
    row: Option<(usize, Modifiers)>,
    resize: Option<(usize, f32)>,
    context: Option<ContextTap>,
    column_begin: Option<usize>,
    column_target: Option<usize>,
    column_end: Option<bool>,
    row_begin: Option<(usize, usize)>,
    row_target: Option<usize>,
    row_end: Option<bool>,
    menu: Option<MenuCommand>,
    menu_open: bool,
    touch_edit: Option<GridFocus>,
    touch_rename: Option<usize>,
}

impl Interaction {
    /// Records a click on a data cell.
    pub fn cell(&mut self, click: (usize, usize, Modifiers)) {
        self.cell = Some(click);
    }

    /// Records a click on a column header.
    pub fn column(&mut self, column: usize, modifiers: Modifiers) {
        self.column = Some((column, modifiers));
    }

    /// Records a click on a row number.
    pub fn row(&mut self, row: usize, modifiers: Modifiers) {
        self.row = Some((row, modifiers));
    }

    /// Records a column drag.
    pub fn resize(&mut self, column: usize, delta: f32) {
        self.resize = Some((column, delta));
    }

    /// Records the element a right click opened a menu on.
    pub fn context(&mut self, tap: ContextTap) {
        self.context = Some(tap);
    }

    /// Records a header drag start.
    pub fn begin_column_drag(&mut self, column: usize) {
        self.column_begin = Some(column);
        self.column_target = Some(column);
    }

    /// Records the insertion slot under a header drag.
    pub fn column_target(&mut self, target: usize) {
        self.column_target = Some(target);
    }

    /// Records a header drag release; `inside` is whether the pointer was over
    /// the grid, so a drag-out does not reorder columns.
    pub fn end_column_drag(&mut self, inside: bool) {
        self.column_end = Some(inside);
    }

    /// Records a row drag start at `row` and visual `column`.
    pub fn begin_row_drag(&mut self, row: usize, column: usize) {
        self.row_begin = Some((row, column));
    }

    /// Records the insertion row under a row drag.
    pub fn row_target(&mut self, target: usize) {
        self.row_target = Some(target);
    }

    /// Records a row drag release; `inside` is whether the pointer was over the
    /// data area, so a drag-out does not move rows.
    pub fn end_row_drag(&mut self, inside: bool) {
        self.row_end = Some(inside);
    }

    /// Records the command a context menu was chosen for.
    pub fn menu(&mut self, command: MenuCommand) {
        self.menu = Some(command);
    }

    /// Records that a grid context menu is open this frame.
    pub fn note_menu_open(&mut self) {
        self.menu_open = true;
    }

    /// Whether a grid context menu was open this frame.
    pub fn is_menu_open(&self) -> bool {
        self.menu_open
    }

    /// Whether a row or column drag begins this frame.
    pub fn starts_drag(&self) -> bool {
        self.row_begin.is_some() || self.column_begin.is_some()
    }

    /// The native edit the touch grid asked for this frame, if any.
    pub fn touch_request(&self) -> Option<GridEditRequest> {
        self.touch_edit
            .map(GridEditRequest::Cell)
            .or_else(|| self.touch_rename.map(GridEditRequest::Column))
    }
}
