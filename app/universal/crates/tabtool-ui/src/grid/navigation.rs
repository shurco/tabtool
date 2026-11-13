//! Keyboard navigation for the focused cell.
//!
//! The mapping is pure enough to read: arrows move one cell, Tab moves through
//! cells and wraps, Home/End reach the row edges, PageUp/PageDown move by the
//! visible row count, and Command+arrows reach the grid edges. Clipboard keys
//! emit [`GridAction`]s; only the dispatcher touches the document.

use egui::{Context, Event, Id, Key, Modifiers};
use tabtool_core::RowSource;

use super::GridAction;
use super::keyboard::{MoveDirection, arrow_direction, cell_id};
use super::options::GridOptions;
use super::selection::{GridFocus, move_focus};
use super::state::GridState;

/// Handles the non-editing grid keys.
#[allow(clippy::too_many_arguments)]
pub(crate) fn handle_navigation(
    ctx: &Context,
    source: &dyn RowSource,
    state: &mut GridState,
    base_id: Id,
    options: &GridOptions,
    actions: &mut Vec<GridAction>,
    scroll_to: &mut Option<GridFocus>,
) {
    let row_count = source.row_count();
    let column_count = source.column_count();
    if row_count == 0 || column_count == 0 {
        return;
    }
    let Some(current) = state.focus else {
        return;
    };

    if let Some(focus) = command_edge(ctx, current, row_count, column_count) {
        select_focus(state, focus);
        *scroll_to = Some(focus);
    } else if let Some((direction, extend)) = arrow_pressed(ctx) {
        move_focus(
            &mut state.selection,
            &mut state.anchor,
            &mut state.focus,
            row_count,
            column_count,
            direction,
            extend,
        );
        *scroll_to = state.focus;
    } else if let Some(focus) = tab_target(ctx, current, row_count, column_count) {
        select_focus(state, focus);
        *scroll_to = Some(focus);
    } else if let Some(focus) = row_edge(ctx, current, column_count) {
        select_focus(state, focus);
        *scroll_to = Some(focus);
    } else if let Some(focus) = page_target(ctx, current, state, options, row_count, column_count) {
        select_focus(state, focus);
        *scroll_to = Some(focus);
    }

    if ctx.input_mut(|input| {
        input.consume_key(Modifiers::NONE, Key::Delete)
            || input.consume_key(Modifiers::NONE, Key::Backspace)
    }) {
        actions.push(GridAction::DeleteSelection);
    }

    if ctx.input_mut(|input| input.consume_key(Modifiers::COMMAND, Key::A)) {
        state.selection.rows = (0..row_count).collect();
        state.selection.columns.clear();
        state.anchor = None;
        actions.push(GridAction::SelectAll);
    }
    clipboard_keys(ctx, actions);

    if let Some(focus) = *scroll_to {
        ctx.memory_mut(|memory| {
            memory.request_focus(cell_id(base_id, focus.row, focus.column));
        });
    }
}

fn select_focus(state: &mut GridState, focus: GridFocus) {
    state
        .selection
        .select_row(focus.row, false, false, &mut state.anchor);
    state.focus = Some(focus);
}

/// Command+arrow edges, most specific first.
fn command_edge(
    ctx: &Context,
    current: GridFocus,
    row_count: usize,
    column_count: usize,
) -> Option<GridFocus> {
    let mut target = None;
    ctx.input_mut(|input| {
        if input.consume_key(Modifiers::COMMAND, Key::ArrowUp) {
            target = Some(GridFocus::new(0, current.column));
        } else if input.consume_key(Modifiers::COMMAND, Key::ArrowDown) {
            target = Some(GridFocus::new(row_count - 1, current.column));
        } else if input.consume_key(Modifiers::COMMAND, Key::ArrowLeft) {
            target = Some(GridFocus::new(current.row, 0));
        } else if input.consume_key(Modifiers::COMMAND, Key::ArrowRight) {
            target = Some(GridFocus::new(current.row, column_count - 1));
        }
    });
    target
}

/// Tab and Shift+Tab, wrapping across rows like the editor's commit move.
fn tab_target(
    ctx: &Context,
    current: GridFocus,
    row_count: usize,
    column_count: usize,
) -> Option<GridFocus> {
    let mut target = None;
    ctx.input_mut(|input| {
        if input.consume_key(Modifiers::SHIFT, Key::Tab) {
            target = Some(GridFocus::new(
                if current.column > 0 {
                    current.row
                } else if current.row > 0 {
                    current.row - 1
                } else {
                    0
                },
                if current.column > 0 {
                    current.column - 1
                } else if current.row > 0 {
                    column_count - 1
                } else {
                    0
                },
            ));
        } else if input.consume_key(Modifiers::NONE, Key::Tab) {
            target = Some(GridFocus::new(
                if current.column + 1 < column_count {
                    current.row
                } else if current.row + 1 < row_count {
                    current.row + 1
                } else {
                    current.row
                },
                if current.column + 1 < column_count {
                    current.column + 1
                } else if current.row + 1 < row_count {
                    0
                } else {
                    current.column
                },
            ));
        }
    });
    target
}

/// Home and End within the focused row.
fn row_edge(ctx: &Context, current: GridFocus, column_count: usize) -> Option<GridFocus> {
    let mut target = None;
    ctx.input_mut(|input| {
        if input.consume_key(Modifiers::NONE, Key::Home) {
            target = Some(GridFocus::new(current.row, 0));
        } else if input.consume_key(Modifiers::NONE, Key::End) {
            target = Some(GridFocus::new(current.row, column_count - 1));
        }
    });
    target
}

/// PageUp/PageDown by the rows the viewport shows.
fn page_target(
    ctx: &Context,
    current: GridFocus,
    state: &GridState,
    options: &GridOptions,
    row_count: usize,
    column_count: usize,
) -> Option<GridFocus> {
    let page = if options.row_height > 0.0 {
        (state.viewport.height() / options.row_height)
            .floor()
            .max(1.0) as usize
    } else {
        1
    };
    let mut target = None;
    ctx.input_mut(|input| {
        if input.consume_key(Modifiers::NONE, Key::PageDown) {
            target = Some(GridFocus::new(
                current.row.saturating_add(page).min(row_count - 1),
                current.column.min(column_count - 1),
            ));
        } else if input.consume_key(Modifiers::NONE, Key::PageUp) {
            target = Some(GridFocus::new(
                current.row.saturating_sub(page),
                current.column.min(column_count - 1),
            ));
        }
    });
    target
}

fn clipboard_keys(ctx: &Context, actions: &mut Vec<GridAction>) {
    let (copy_event, paste_event) = ctx.input(|input| {
        let copy = input
            .events
            .iter()
            .any(|event| matches!(event, Event::Copy));
        let paste = input
            .events
            .iter()
            .any(|event| matches!(event, Event::Paste(_)));
        (copy, paste)
    });
    if copy_event {
        actions.push(GridAction::Copy);
    }
    if paste_event {
        actions.push(GridAction::Paste);
    }
    if !copy_event && ctx.input_mut(|input| input.consume_key(Modifiers::COMMAND, Key::C)) {
        actions.push(GridAction::Copy);
    }
    if !paste_event && ctx.input_mut(|input| input.consume_key(Modifiers::COMMAND, Key::V)) {
        actions.push(GridAction::Paste);
    }
}

/// The arrow direction and whether Shift extends the selection.
fn arrow_pressed(ctx: &Context) -> Option<(MoveDirection, bool)> {
    let shift = ctx.input(|input| {
        input
            .events
            .iter()
            .find_map(|event| match event {
                Event::Key {
                    key,
                    pressed: true,
                    modifiers,
                    ..
                } if arrow_direction(*key).is_some() => Some(modifiers.shift),
                _ => None,
            })
            .unwrap_or(input.modifiers.shift)
    });
    for key in [
        Key::ArrowUp,
        Key::ArrowDown,
        Key::ArrowLeft,
        Key::ArrowRight,
    ] {
        let modifiers = if shift {
            Modifiers::SHIFT
        } else {
            Modifiers::NONE
        };
        if ctx.input_mut(|input| input.consume_key(modifiers, key)) {
            return arrow_direction(key).map(|direction| (direction, shift));
        }
    }
    None
}
