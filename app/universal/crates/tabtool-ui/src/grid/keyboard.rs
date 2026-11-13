//! Key mapping and keyboard handling for the grid and its single-cell editor.
//!
//! [`editor_action`], [`starts_editing`] and [`arrow_direction`] are pure so
//! the key rules are testable without an egui context. [`handle_keys`] reads
//! the pressed keys once per frame and applies them to the persistent state.

use egui::{Context, Event, Id, Key, Modifiers};
use tabtool_core::RowSource;

use super::GridAction;
use super::editor::{self, CellEditor};
use super::selection::{GridFocus, move_focus};
use super::state::GridState;

/// A direction the editor moves after committing.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MoveDirection {
    /// One row up.
    Up,
    /// One row down.
    Down,
    /// One column left.
    Left,
    /// One column right.
    Right,
}

/// What an editing key does.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EditorKeyAction {
    /// Commit the text and move the focus in `MoveDirection`.
    CommitAndMove(MoveDirection),
    /// Discard the editor without an action.
    Cancel,
}

/// The editor action for `key`, or `None` when the editor keeps the key.
///
/// Return commits and moves down, Tab commits and moves right, Shift+Tab
/// commits and moves left, and Escape cancels. All other keys, including the
/// arrows and text input, stay with the text field.
pub fn editor_action(key: Key, shift: bool) -> Option<EditorKeyAction> {
    match key {
        Key::Enter => Some(EditorKeyAction::CommitAndMove(MoveDirection::Down)),
        Key::Tab if shift => Some(EditorKeyAction::CommitAndMove(MoveDirection::Left)),
        Key::Tab => Some(EditorKeyAction::CommitAndMove(MoveDirection::Right)),
        Key::Escape => Some(EditorKeyAction::Cancel),
        _ => None,
    }
}

/// Whether `key` starts editing the focused cell.
pub fn starts_editing(key: Key) -> bool {
    key == Key::Enter
}

/// The arrow direction for `key`, or `None` for other keys.
pub fn arrow_direction(key: Key) -> Option<MoveDirection> {
    match key {
        Key::ArrowUp => Some(MoveDirection::Up),
        Key::ArrowDown => Some(MoveDirection::Down),
        Key::ArrowLeft => Some(MoveDirection::Left),
        Key::ArrowRight => Some(MoveDirection::Right),
        _ => None,
    }
}

/// The stable widget id of one data cell.
pub(crate) fn cell_id(base_id: Id, row: usize, column: usize) -> Id {
    base_id.with(("cell", row, column))
}

/// Handles the editor and grid keyboard shortcuts.
#[allow(clippy::too_many_arguments)]
pub(crate) fn handle_keys(
    ctx: &Context,
    source: &dyn RowSource,
    state: &mut GridState,
    base_id: Id,
    editor_id: Id,
    editable: bool,
    active_editor: &mut Option<CellEditor>,
    actions: &mut Vec<GridAction>,
    scroll_to: &mut Option<GridFocus>,
) {
    let editing_now = active_editor.is_some() || state.editing.is_some();
    if editing_now {
        // egui clears keyboard focus when Escape is pressed before the frame's
        // widgets run, so Escape must cancel even though no text field is
        // focused any more.
        if ctx.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Escape)) {
            *active_editor = None;
            state.editing = None;
            return;
        }
        if !ctx.text_edit_focused() {
            return;
        }
        let editing = active_editor.take().or_else(|| state.editing.take());
        let Some(editing) = editing else {
            return;
        };
        let pressed = ctx.input_mut(|input| {
            if input.consume_key(Modifiers::SHIFT, Key::Tab) {
                return Some((Key::Tab, true));
            }
            if input.consume_key(Modifiers::NONE, Key::Tab) {
                return Some((Key::Tab, false));
            }
            if input.consume_key(Modifiers::NONE, Key::Enter) {
                return Some((Key::Enter, false));
            }
            None
        });
        let action = pressed.and_then(|(key, shift)| editor_action(key, shift));
        match action {
            None => {
                *active_editor = Some(editing);
            }
            Some(EditorKeyAction::Cancel) => {
                state.editing = None;
            }
            Some(EditorKeyAction::CommitAndMove(direction)) => {
                actions.push(editing.commit_action());
                if let Some(next) = editor::move_after_commit(
                    &editing,
                    direction,
                    source.row_count(),
                    source.column_count(),
                ) {
                    state
                        .selection
                        .select_row(next.row, false, false, &mut state.anchor);
                    state.focus = Some(next);
                    *scroll_to = Some(next);
                    ctx.memory_mut(|memory| {
                        memory.request_focus(cell_id(base_id, next.row, next.column));
                    });
                }
                state.editing = None;
            }
        }
        return;
    }

    let focused = ctx.memory(|memory| memory.focused());
    let grid_focused = state
        .focus
        .is_some_and(|focus| focused == Some(cell_id(base_id, focus.row, focus.column)));
    if !grid_focused {
        return;
    }

    if editable
        && starts_editing(Key::Enter)
        && ctx.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Enter))
    {
        let target = state.focus.or_else(|| {
            state
                .selection
                .rows
                .first()
                .map(|row| GridFocus::new(*row, 0))
        });
        if let Some(target) = target
            && target.row < source.row_count()
            && target.column < source.column_count()
        {
            let text = source.cell(target.row, target.column);
            state.start_editing(target.row, target.column, text);
            ctx.memory_mut(|memory| memory.request_focus(editor_id));
            state.pending_editor_focus = true;
        }
        return;
    }

    if let Some((direction, extend)) = arrow_pressed(ctx) {
        move_focus(
            &mut state.selection,
            &mut state.anchor,
            &mut state.focus,
            source.row_count(),
            source.column_count(),
            direction,
            extend,
        );
        *scroll_to = state.focus;
    }
    if let Some(focus) = *scroll_to {
        ctx.memory_mut(|memory| {
            memory.request_focus(cell_id(base_id, focus.row, focus.column));
        });
    }
}

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
