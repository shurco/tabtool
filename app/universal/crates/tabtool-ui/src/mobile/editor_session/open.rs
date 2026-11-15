//! Opening, placing and moving the native editor over a cell or a header.

use egui::Rect;

use super::EditorSession;
use crate::grid::{CellEditor, GridEditRequest, GridFocus, MoveDirection, move_after_commit};
use crate::mobile::TouchTextInputRequest;
use crate::mobile::editor_placement::{placement, scroll_target};
use crate::mobile::editor_state::{MobileEdit, MobileEditTarget};
use crate::mobile::editor_targets::{cell_text, header};

impl<'a> EditorSession<'a> {
    /// Opens the requested native editor.
    pub fn open_request(&mut self, request: GridEditRequest) {
        match request {
            GridEditRequest::Cell(focus) => self.begin_cell_edit(focus),
            GridEditRequest::Column(column) => self.begin_column_edit(column),
        }
    }

    /// Places a pending editor once its cell is fully inside the grid area.
    ///
    /// Called after the grid draws each frame: the viewport and the grid
    /// rectangle are current by then. An editor that is already open follows
    /// its cell when the keyboard or a scroll moves it.
    pub fn maybe_open(&mut self) {
        let Some(edit) = &self.editor.editing else {
            return;
        };
        if edit.awaiting_commit {
            return;
        }
        let target = edit.target;
        let opened = edit.opened;
        let text = edit.text.clone();
        let Some(place) = placement(
            self.document,
            self.state,
            self.editor,
            &self.options,
            target,
        ) else {
            self.editor.editing = None;
            return;
        };
        let oversized = place.rect.width() > place.visible.width()
            || place.rect.height() > place.visible.height();
        if !oversized && !place.visible.contains_rect(place.rect) {
            if let Some(focus) = scroll_target(self.document, self.state, target) {
                self.state.scroll_to_cell(focus);
            }
            return;
        }
        let rect = place.rect;
        if opened {
            if self.editor.last_editor_rect != Some(rect) {
                self.text_input.move_to(rect);
                self.editor.last_editor_rect = Some(rect);
            }
        } else {
            self.place(text, rect);
        }
    }

    fn begin_cell_edit(&mut self, focus: GridFocus) {
        if !self.document.can_edit_cells() {
            return;
        }
        let Some((text, row_count, column_count)) = cell_text(self.document, focus) else {
            return;
        };
        if focus.row >= row_count || focus.column >= column_count {
            return;
        }
        self.state
            .selection
            .select_row(focus.row, false, false, &mut self.state.anchor);
        self.state.focus = Some(focus);
        self.editor.editing = Some(MobileEdit {
            target: MobileEditTarget::Cell {
                row: focus.row,
                column: focus.column,
            },
            text,
            opened: false,
            awaiting_commit: false,
        });
        self.maybe_open();
    }

    fn begin_column_edit(&mut self, column: usize) {
        if !self.document.can_rename_column(column) {
            return;
        }
        let Some((id, text)) = header(self.document, column) else {
            return;
        };
        self.state
            .selection
            .select_column(column, false, false, &mut self.state.anchor);
        self.editor.editing = Some(MobileEdit {
            target: MobileEditTarget::Column(id),
            text,
            opened: false,
            awaiting_commit: false,
        });
        self.maybe_open();
    }

    pub(super) fn move_after(&mut self, origin: Option<&MobileEdit>, direction: MoveDirection) {
        let Some(MobileEdit {
            target: MobileEditTarget::Cell { row, column },
            ..
        }) = origin
        else {
            return;
        };
        let Some((_, row_count, column_count)) =
            cell_text(self.document, GridFocus::new(*row, *column))
        else {
            return;
        };
        let editor = CellEditor::new(*row, *column, "");
        if let Some(next) = move_after_commit(&editor, direction, row_count, column_count) {
            self.begin_cell_edit(next);
        }
    }

    fn place(&mut self, text: String, rect: Rect) {
        // A quoted cell may contain line breaks. The platform editor is
        // single-line for ordinary cells, matching the desktop field, and
        // accepts breaks only when the cell already has them.
        let multiline = text.contains(['\n', '\r']);
        self.text_input.open(TouchTextInputRequest {
            rect,
            text,
            multiline,
        });
        self.editor.last_editor_rect = Some(rect);
        if let Some(edit) = &mut self.editor.editing {
            edit.opened = true;
        }
    }
}
