//! The single-cell text editor overlay.

use egui::{
    Align2, Context, FontId, Frame, Id, Margin, Rect, Stroke, StrokeKind, TextEdit, Ui,
    text::{CCursor, CCursorRange},
    text_edit::TextEditState,
    vec2,
};

use crate::grid::GridAction;
use crate::grid::keyboard::MoveDirection;
use crate::grid::selection::GridFocus;
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

/// Resets the text field's cursor to `caret` and gives it keyboard focus.
///
/// Called once when an editor starts, so a stale caret from the previous cell
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

/// Draws the editor over `rect`, clipped to `clip`.
///
/// Returns the text field's response. The editor stores the caret from
/// [`TextEditState`]; the render pass decides whether a focus loss must commit.
pub(crate) fn show(
    ui: &mut Ui,
    id: Id,
    rect: Rect,
    clip: Rect,
    editor: &mut CellEditor,
    name: &str,
    right_aligned: bool,
) -> egui::Response {
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
        TextEdit::singleline(&mut editor.text)
            .id(id)
            .frame(Frame::NONE)
            .margin(Margin::symmetric(6, 2))
            .font(FontId::proportional(theme.typography.body))
            .text_color(theme.colors.label)
            .align(align)
            .desired_width(width)
            .min_size(vec2(rect.width(), rect.height())),
    );
    let response = response.accessible_name(name);

    if let Some(state) = TextEditState::load(ui.ctx(), id)
        && let Some(range) = state.cursor.char_range()
    {
        editor.caret = range.primary.index.0;
    }
    response
}
