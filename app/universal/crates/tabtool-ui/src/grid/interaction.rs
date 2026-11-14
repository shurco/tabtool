//! Pointer gestures accumulated during the render pass and applied after it.
//!
//! Keeping the mutation here means painting and hit testing only read the grid
//! state; one place then updates selection, starts drags, opens the rename
//! editor and emits the actions a gesture requested.

use std::collections::BTreeSet;

use egui::{Context, Id, Key, Modifiers};
use tabtool_core::RowSource;

use super::GridAction;
use super::context::{self, MenuCommand};
use super::drag::{ColumnDrag, RowDrag, column_drop_action, row_drop_action};
use super::keyboard::cell_id;
use super::options::GridOptions;
use super::selection::{GridAnchor, GridElement, GridFocus, GridSelection};
use super::state::GridState;
use super::touch::{GridEditRequest, GridTouch, TouchGesture};

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

    /// Applies the gesture to the persistent state and collects its actions.
    #[allow(clippy::too_many_arguments)]
    pub fn apply(
        &mut self,
        ctx: &Context,
        base_id: Id,
        state: &mut GridState,
        source: &dyn RowSource,
        options: &GridOptions,
        was_editing: bool,
        actions: &mut Vec<GridAction>,
    ) {
        if (state.column_drag.is_some() || state.row_drag.is_some())
            && ctx.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Escape))
        {
            state.column_drag = None;
            state.row_drag = None;
        }
        if let Some((column, delta)) = self.resize.take() {
            state.resize_column(source, column, delta, options);
        }

        let mut clicked = false;
        if let Some(tap) = self.context.take() {
            clicked = true;
            apply_context_selection(state, source, tap);
        }
        if let Some((row, column, modifiers)) = self.cell.take() {
            clicked = true;
            let tapped = GridFocus::new(row, column);
            let gesture = GridTouch::new().tap(GridElement::Cell, state.focus, Some(tapped));
            state
                .selection
                .select_row(row, modifiers.shift, modifiers.command, &mut state.anchor);
            state.focus = Some(tapped);
            if gesture == TouchGesture::EditCell
                && !was_editing
                && !modifiers.shift
                && !modifiers.command
            {
                if options.touch {
                    // The owner places the platform editor; the grid must not
                    // open an in-window text field on a touch device.
                    if options.editable {
                        self.touch_edit = Some(tapped);
                    }
                } else if options.editable {
                    let text = source.cell(row, column);
                    state.start_editing(row, column, text);
                }
            }
        } else if let Some((column, modifiers)) = self.column.take() {
            clicked = true;
            let row = state
                .focus
                .map(|focus| focus.row)
                .or_else(|| state.selection.rows.first().copied())
                .unwrap_or(0);
            if GridTouch::new().tap(GridElement::ColumnHeader, state.focus, None)
                == TouchGesture::SelectColumn
            {
                state.selection.select_column(
                    column,
                    modifiers.shift,
                    modifiers.command,
                    &mut state.anchor,
                );
            }
            state.focus = if options.touch {
                // A touch header tap selects the whole column. Keeping a cell
                // focus would draw a cell border under the column and make the
                // next tap on that cell edit instead of select its row.
                None
            } else {
                (source.row_count() > 0)
                    .then_some(GridFocus::new(row.min(source.row_count() - 1), column))
            };
            state.editing = None;
        } else if let Some((row, modifiers)) = self.row.take() {
            clicked = true;
            if GridTouch::new().tap(GridElement::RowNumber, state.focus, None)
                == TouchGesture::SelectRowNumber
            {
                state.selection.select_row(
                    row,
                    modifiers.shift,
                    modifiers.command,
                    &mut state.anchor,
                );
            }
            state.focus = (source.column_count() > 0).then_some(GridFocus::new(row, 0));
            state.editing = None;
        }

        if let Some((row, column)) = self.row_begin.take() {
            begin_row_drag(state, source, row, column);
            clicked = true;
        }
        if let Some(target) = self.row_target.take()
            && let Some(drag) = &mut state.row_drag
        {
            drag.target = Some(target);
        }
        if let Some(inside) = self.row_end.take() {
            let drag = state.row_drag.take();
            if inside
                && let Some(drag) = drag
                && let Some(action) = row_drop_action(&drag, source.row_count())
            {
                actions.push(action);
            }
        }

        if let Some(column) = self.column_begin.take() {
            state.column_drag = ColumnDrag::new(source, column);
            state.row_drag = None;
        }
        if let Some(target) = self.column_target.take()
            && let Some(drag) = &mut state.column_drag
        {
            drag.target = target;
        }
        if let Some(inside) = self.column_end.take() {
            let drag = state.column_drag.take();
            if inside
                && let Some(drag) = drag
                && let Some(action) = column_drop_action(source, &drag)
            {
                actions.push(action);
            }
        }

        if let Some(command) = self.menu.take() {
            match command {
                MenuCommand::Run(action) => actions.push(action),
                MenuCommand::StartRename { column } => {
                    if options.touch {
                        self.touch_rename = Some(column);
                    } else {
                        state.start_renaming(source, column);
                    }
                }
            }
        }

        if clicked
            && state.editing.is_none()
            && state.header_editing.is_none()
            && let Some(focus) = state.focus
        {
            ctx.memory_mut(|memory| {
                memory.request_focus(cell_id(base_id, focus.row, focus.column));
            });
        }
    }
}

fn apply_context_selection(state: &mut GridState, source: &dyn RowSource, tap: ContextTap) {
    state.selection =
        context::context_selection(tap.element, tap.row, tap.column, &state.selection);
    state.anchor = match tap.element {
        GridElement::ColumnHeader => Some(GridAnchor::Column(tap.column)),
        GridElement::Cell | GridElement::RowNumber => Some(GridAnchor::Row(tap.row)),
        GridElement::Corner => None,
    };
    state.focus = match tap.element {
        GridElement::Cell => Some(GridFocus::new(tap.row, tap.column)),
        GridElement::RowNumber => Some(GridFocus::new(tap.row, 0)),
        GridElement::ColumnHeader => {
            let row = state
                .focus
                .map(|focus| focus.row)
                .unwrap_or(0)
                .min(source.row_count().saturating_sub(1));
            (source.row_count() > 0).then_some(GridFocus::new(row, tap.column))
        }
        GridElement::Corner => None,
    };
    state.editing = None;
    state.header_editing = None;
}

fn begin_row_drag(state: &mut GridState, source: &dyn RowSource, row: usize, column: usize) {
    let rows: BTreeSet<usize> = if state.selection.rows.contains(&row) {
        state.selection.rows.clone()
    } else {
        [row].into_iter().collect()
    };
    state.selection = GridSelection {
        rows: rows.clone(),
        columns: BTreeSet::new(),
    };
    state.anchor = Some(GridAnchor::Row(row));
    state.focus = Some(GridFocus::new(row, column));
    state.editing = None;
    state.header_editing = None;
    state.row_drag = RowDrag::new(source, rows);
    state.column_drag = None;
}
