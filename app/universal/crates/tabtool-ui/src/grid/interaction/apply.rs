//! Applying the accumulated gesture to the persistent grid state.

use std::collections::BTreeSet;

use egui::{Context, Id, Key, Modifiers};
use tabtool_core::RowSource;

use super::{ContextTap, Interaction};
use crate::grid::GridAction;
use crate::grid::context::{self, MenuCommand};
use crate::grid::drag::{ColumnDrag, RowDrag, column_drop_action, row_drop_action};
use crate::grid::keyboard::cell_id;
use crate::grid::options::GridOptions;
use crate::grid::selection::{GridAnchor, GridElement, GridFocus, GridSelection};
use crate::grid::state::GridState;
use crate::grid::touch::{GridTouch, TouchGesture};

impl Interaction {
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
