//! The touch editor's edit session: native text commits, accessory moves and
//! the document actions that must wait for a commit.
//!
//! A platform editor reports its text on a later frame, so a tap, a toolbar
//! command or a contextual action that arrives while a cell is being edited is
//! queued here and runs only after the commit lands. That ordering keeps typed
//! text in the cell it came from and applies every edit exactly once.

use tabtool_core::Locale;
use tabtool_document::Document;

use super::editor_state::{MobileEdit, MobileEditTarget, MobileEditorState, PendingWork};
use super::editor_targets::column_of;
use super::{MobileToolbarResponse, TouchTextInput, TouchTextInputOutcome};
use crate::document::{
    apply_grid_action, poll_large_state, sync_selection_to_document, sync_selection_to_grid,
};
use crate::grid::{Clipboard, GridAction, GridOptions, GridState};

mod open;

/// The touch editor's document-backed operations.
pub(crate) struct EditorSession<'a> {
    pub document: &'a mut Document,
    pub state: &'a mut GridState,
    pub editor: &'a mut MobileEditorState,
    pub text_input: &'a mut dyn TouchTextInput,
    pub clipboard: &'a mut dyn Clipboard,
    pub locale: &'a Locale,
    pub decimal_separator: &'a str,
    pub options: GridOptions,
}

impl<'a> EditorSession<'a> {
    /// Polls the large index and at-rest staleness.
    pub fn poll_large(&mut self, ctx: &egui::Context) {
        poll_large_state(self.document, &mut self.editor.last_stale_check, ctx);
    }

    /// Takes the document's one-shot notice into the editor state.
    pub fn take_notice(&mut self) {
        if let Some(notice) = self.document.take_notice() {
            self.editor.notice = Some(notice);
        }
    }

    /// Mirrors the document selection into the grid and drops a stale focus.
    pub fn sync_selection(&mut self) {
        sync_selection_to_grid(self.document, self.state);
        if self.document.selection().rows.is_empty()
            && self.document.selection().columns.is_empty()
            && self.state.selection.rows.is_empty()
            && self.state.selection.columns.is_empty()
        {
            self.state.focus = None;
        }
    }

    /// Copies the grid selection into the document for the next frame.
    pub fn sync_document(&mut self) {
        sync_selection_to_document(self.document, self.state);
    }

    /// Applies the platform editor's outcome.
    pub fn poll_outcome(&mut self) {
        match self.text_input.poll() {
            Some(TouchTextInputOutcome::Committed(text)) => {
                let Some(edit) = self.editor.editing.take() else {
                    return;
                };
                self.apply_text(&edit, &text);
                let work = self.editor.pending.take().unwrap_or_default();
                self.run_work(work, Some(&edit));
            }
            Some(TouchTextInputOutcome::Cancelled) => {
                self.editor.editing = None;
                self.editor.pending = None;
            }
            None => {
                let opened = self.editor.editing.as_ref().is_some_and(|edit| edit.opened);
                if opened && !self.text_input.is_active() {
                    // The platform closed the session without an outcome (an
                    // open that failed or an activity recreation). Drop the
                    // deferred work instead of applying stale text later.
                    self.editor.editing = None;
                    self.editor.pending = None;
                }
            }
        }
    }

    /// Runs `work` now, or after the open edit commits.
    pub fn submit(&mut self, work: PendingWork) {
        let Some(edit) = &self.editor.editing else {
            self.run_work(work, None);
            return;
        };
        let awaiting_commit = edit.awaiting_commit;
        if awaiting_commit || self.text_input.is_active() {
            self.request_commit();
            if !work.is_empty() {
                self.merge_pending(work);
            }
        } else {
            // The platform session vanished before it opened; its work can
            // run without a commit.
            self.editor.editing = None;
            self.run_work(work, None);
        }
    }

    /// Applies one grid action immediately.
    pub fn apply_action(&mut self, action: GridAction) {
        let _ = apply_grid_action(
            self.document,
            self.state,
            action,
            self.clipboard,
            self.locale,
            self.decimal_separator,
        );
    }

    /// Requests a commit from the open native editor.
    pub fn request_commit(&mut self) {
        if let Some(edit) = &mut self.editor.editing
            && !edit.awaiting_commit
        {
            self.text_input.commit();
            edit.awaiting_commit = true;
        }
    }

    fn merge_pending(&mut self, work: PendingWork) {
        let pending = self.editor.pending.get_or_insert_with(PendingWork::default);
        pending.actions.extend(work.actions);
        pending.toolbar.merge(work.toolbar);
        if work.open.is_some() {
            pending.open = work.open;
        }
        if work.move_to.is_some() {
            pending.move_to = work.move_to;
        }
        pending.undo |= work.undo;
        pending.show_format_bar |= work.show_format_bar;
    }

    fn run_work(&mut self, work: PendingWork, origin: Option<&MobileEdit>) {
        for action in work.actions {
            self.apply_action(action);
        }
        if work.undo {
            self.document.undo();
        }
        if let Some(request) = work.open {
            self.open_request(request);
        }
        if let Some(direction) = work.move_to {
            self.move_after(origin, direction);
        }
        let mut toolbar = work.toolbar;
        if toolbar.undo {
            self.document.undo();
            toolbar.undo = false;
        }
        if toolbar.redo {
            self.document.redo();
            toolbar.redo = false;
        }
        if !toolbar.is_empty() {
            let deferred = self
                .editor
                .deferred_toolbar
                .get_or_insert_with(MobileToolbarResponse::default);
            deferred.merge(toolbar);
        }
        self.editor.deferred_format_bar |= work.show_format_bar;
    }

    fn apply_text(&mut self, edit: &MobileEdit, text: &str) {
        match edit.target {
            MobileEditTarget::Cell { row, column } => {
                self.apply_action(GridAction::CommitCell {
                    row,
                    column,
                    text: text.to_owned(),
                });
            }
            MobileEditTarget::Column(id) => {
                let Some(column) = column_of(self.document, id) else {
                    return;
                };
                self.apply_action(GridAction::RenameColumn {
                    column,
                    name: text.to_owned(),
                });
            }
        }
    }
}
