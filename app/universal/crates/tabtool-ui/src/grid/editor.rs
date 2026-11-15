//! The single-cell text editor and the inline header rename editor.

use egui::{
    Align2, Context, FontId, Frame, Id, Key, Margin, Modifiers, Rect, Response, Stroke, StrokeKind,
    TextEdit, Ui,
    text::{CCursor, CCursorRange},
    text_edit::TextEditState,
    vec2,
};
use tabtool_core::{ColumnId, RowSource};

use crate::grid::GridAction;
use crate::grid::keyboard::MoveDirection;
use crate::grid::selection::GridFocus;
use crate::grid::state::GridState;
use crate::theme::{Theme, current};

/// The cell currently being edited.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CellEditor {
    /// Display row.
    pub row: usize,
    /// Visual column.
    pub column: usize,
    /// The field's current text.
    pub text: String,
    /// The caret position in characters.
    pub caret: usize,
}

impl CellEditor {
    /// Starts an editor seeded with `text` and the caret at its end.
    pub fn new(row: usize, column: usize, text: impl Into<String>) -> Self {
        let text = text.into();
        let caret = text.chars().count();
        Self {
            text,
            caret,
            row,
            column,
        }
    }

    /// The action that writes this editor's text into the document.
    pub fn commit_action(&self) -> GridAction {
        GridAction::CommitCell {
            row: self.row,
            column: self.column,
            text: self.text.clone(),
        }
    }
}

/// The column header being renamed.
///
/// The identity is the column, not its position: a reorder before the edit ends
/// still commits to the same column.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HeaderEditor {
    /// The stable identity of the renamed column.
    pub column: ColumnId,
    /// The field's current text.
    pub text: String,
}

/// The focus after committing `editor` and moving in `direction`.
///
/// Down clamps to the last row, right wraps to the next row's first column
/// (and stays put at the end), and left wraps backwards.
pub(crate) fn move_after_commit(
    editor: &CellEditor,
    direction: MoveDirection,
    row_count: usize,
    column_count: usize,
) -> Option<GridFocus> {
    if row_count == 0 || column_count == 0 {
        return None;
    }
    let last_row = row_count - 1;
    let last_column = column_count - 1;
    let (row, column) = match direction {
        MoveDirection::Up => (editor.row.saturating_sub(1), editor.column),
        MoveDirection::Down => ((editor.row + 1).min(last_row), editor.column),
        MoveDirection::Right if editor.column < last_column => (editor.row, editor.column + 1),
        MoveDirection::Right if editor.row < last_row => (editor.row + 1, 0),
        MoveDirection::Right => (editor.row, last_column),
        MoveDirection::Left if editor.column > 0 => (editor.row, editor.column - 1),
        MoveDirection::Left if editor.row > 0 => (editor.row - 1, last_column),
        MoveDirection::Left => (editor.row, 0),
    };
    Some(GridFocus::new(row.min(last_row), column.min(last_column)))
}

/// Handles the header editor's commit and cancel keys.
pub(crate) fn handle_header_keys(
    ctx: &Context,
    source: &dyn RowSource,
    state: &mut GridState,
    actions: &mut Vec<GridAction>,
) {
    if ctx.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Escape)) {
        state.header_editing = None;
        state.pending_header_focus = false;
        return;
    }
    if !ctx.text_edit_focused() {
        return;
    }
    if state.ime_composing || state.ime_commit_guard {
        // An IME Enter commits the composition, not the header. Consume it so
        // the text field keeps focus and receives the composition commit.
        if ctx.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Enter)) {
            state.ime_commit_guard = false;
        }
        return;
    }
    if ctx.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Enter)) {
        commit_header_editing(source, state, actions);
    }
}

/// Commits the header editor, resolving the column by identity.
///
/// A column that no longer exists cancels without an action.
pub(crate) fn commit_header_editing(
    source: &dyn RowSource,
    state: &mut GridState,
    actions: &mut Vec<GridAction>,
) {
    let Some(editing) = state.header_editing.take() else {
        return;
    };
    state.pending_header_focus = false;
    let Some(column) = source
        .column_ids()
        .iter()
        .position(|id| *id == editing.column)
    else {
        return;
    };
    actions.push(GridAction::RenameColumn {
        column,
        name: editing.text,
    });
}

/// Resets the text field's cursor to `caret` and gives it keyboard focus.
///
/// Called once when an editor starts, so a stale caret from the previous field
/// cannot move the cursor into the middle of the new text.
pub(crate) fn focus_with_caret_end(ctx: &Context, id: Id, caret: usize) {
    let mut state = TextEditState::load(ctx, id).unwrap_or_default();
    state
        .cursor
        .set_char_range(Some(CCursorRange::one(CCursor::new(caret))));
    // A local field-editor undo would restore the previous cell's text; cell
    // edits are undone through the document's own history instead.
    state.clear_undoer();
    state.store(ctx, id);
    ctx.memory_mut(|memory| memory.request_focus(id));
}

/// Draws the cell editor over `rect`, clipped to `clip`.
pub(crate) fn show(
    ui: &mut Ui,
    id: Id,
    rect: Rect,
    clip: Rect,
    editor: &mut CellEditor,
    name: &str,
    right_aligned: bool,
) -> Response {
    let theme: Theme = current(ui.ctx());
    let response = field(
        ui,
        id,
        rect,
        clip,
        &mut editor.text,
        name,
        right_aligned,
        theme.typography.body,
    );

    if let Some(state) = TextEditState::load(ui.ctx(), id)
        && let Some(range) = state.cursor.char_range()
    {
        editor.caret = range.primary.index.0;
    }
    response
}

/// Draws the header rename editor over `rect`, clipped to `clip`.
pub(crate) fn show_header(
    ui: &mut Ui,
    id: Id,
    rect: Rect,
    clip: Rect,
    editor: &mut HeaderEditor,
    name: &str,
) -> Response {
    let theme: Theme = current(ui.ctx());
    field(
        ui,
        id,
        rect,
        clip,
        &mut editor.text,
        name,
        false,
        theme.typography.caption,
    )
}

#[allow(clippy::too_many_arguments)]
fn field(
    ui: &mut Ui,
    id: Id,
    rect: Rect,
    clip: Rect,
    text: &mut String,
    name: &str,
    right_aligned: bool,
    font_size: f32,
) -> Response {
    let theme: Theme = current(ui.ctx());
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect));
    child.set_clip_rect(rect.intersect(clip));
    let painter = child.painter();
    painter.rect_filled(rect, 0, theme.colors.control_background);
    painter.rect_stroke(
        rect,
        0,
        Stroke::new(1.0, theme.colors.accent),
        StrokeKind::Inside,
    );

    let width = (rect.width() - 12.0).max(0.0);
    let align = if right_aligned {
        Align2::RIGHT_CENTER
    } else {
        Align2::LEFT_CENTER
    };
    let response = child.add(
        TextEdit::singleline(text)
            .id(id)
            .frame(Frame::NONE)
            .margin(Margin::symmetric(6, 2))
            .font(FontId::proportional(font_size))
            .text_color(theme.colors.label)
            .align(align)
            .desired_width(width)
            .min_size(vec2(rect.width(), rect.height())),
    );
    response.accessible_name(name)
}
