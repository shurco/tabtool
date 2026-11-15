//! Menu dispatch for a document window.

use std::collections::BTreeSet;

use egui::Context;
use tabtool_core::SortDirection;
use tabtool_document::GridSelection;

use crate::documents::{CloseChoice, WindowRequest, WindowState};
use crate::files::RevertOutcome;
use crate::menu::{Capabilities, MenuAction};

impl WindowState {
    /// Runs every request the window UI queued since the last tick.
    pub(crate) fn drain_requests(&mut self, ctx: &Context) {
        for request in std::mem::take(&mut self.requests) {
            match request {
                WindowRequest::Menu(action) => self.handle_menu(action, ctx),
                WindowRequest::Close(choice) => match choice {
                    CloseChoice::Save => {
                        if self.save_for_close() == crate::documents::CloseSave::Saved {
                            self.should_close = true;
                        }
                        self.close_prompt = false;
                    }
                    CloseChoice::DontSave => {
                        self.close_prompt = false;
                        self.should_close = true;
                    }
                    CloseChoice::Cancel => self.close_prompt = false,
                },
                WindowRequest::Revert(true) => self.revert_now(),
                WindowRequest::Revert(false) => self.revert_prompt = false,
                WindowRequest::Open(choice) => self.answer_open(choice),
                WindowRequest::CancelSave => self.cancel_save(),
            }
        }
    }

    fn handle_menu(&mut self, action: MenuAction, ctx: &Context) {
        if action.is_app_action() {
            self.app_actions.push(action);
            return;
        }
        self.commit_editor();
        match action {
            MenuAction::Close => match crate::documents::close_decision(&self.document) {
                crate::documents::CloseDecision::Close => self.should_close = true,
                crate::documents::CloseDecision::Ask => self.close_prompt = true,
                crate::documents::CloseDecision::Busy => self.busy = true,
            },
            MenuAction::Save => {
                let _ = self.save_auto();
            }
            MenuAction::SaveAs => {
                let _ = self.save_with_dialog();
            }
            MenuAction::RevertToSaved => match self.file_ops.revert(&mut self.document, false) {
                RevertOutcome::NeedsConfirmation => self.revert_prompt = true,
                RevertOutcome::Failed(error) => self.issue = Some(error),
                RevertOutcome::Reverted | RevertOutcome::Unavailable => {}
            },
            MenuAction::Export => self.open_export_panel(),
            MenuAction::RenameColumn => self.rename_selected_column(),
            MenuAction::Paste => self.chrome.request_paste(ctx, self.viewport_id),
            command => self.perform_document_command(command, ctx),
        }
    }

    /// Starts the inline header editor for the selected column.
    fn rename_selected_column(&mut self) {
        let Some(column) = self.document.selection().columns.first().copied() else {
            return;
        };
        let Some(source) = self.document.row_source() else {
            return;
        };
        self.grid.start_renaming(source, column);
    }

    fn revert_now(&mut self) {
        self.revert_prompt = false;
        if let RevertOutcome::Failed(error) = self.file_ops.revert(&mut self.document, true) {
            self.issue = Some(error);
        }
    }

    fn perform_document_command(&mut self, command: MenuAction, ctx: &Context) {
        match command {
            MenuAction::Undo => {
                self.document.undo();
            }
            MenuAction::Redo => {
                self.document.redo();
            }
            MenuAction::Cut => {
                self.copy_selection(ctx);
                self.document.delete_selection();
            }
            MenuAction::Copy => self.copy_selection(ctx),
            MenuAction::DeleteSelection => self.document.delete_selection(),
            MenuAction::SelectAll => {
                if !Capabilities::of(&self.document).select_all {
                    return;
                }
                if let Some(table) = self.document.table() {
                    let rows: BTreeSet<usize> = (0..table.row_count()).collect();
                    *self.document.selection_mut() = GridSelection {
                        rows,
                        columns: BTreeSet::new(),
                    };
                }
            }
            MenuAction::AddRowAbove => self.document.add_row_above_selection(),
            MenuAction::AddRowBelow => self.document.add_row_below_selection(),
            MenuAction::AddColumnLeft => self.document.add_column_left_of_selection(),
            MenuAction::AddColumnRight => self.document.add_column_right_of_selection(),
            MenuAction::DeleteRows => {
                let rows = self.document.selection().rows.clone();
                self.document.delete_rows(&rows);
            }
            MenuAction::DeleteColumns => {
                let columns = self.document.selection().columns.clone();
                self.document.delete_columns(&columns);
            }
            MenuAction::SortAscending => {
                self.sort_selected(SortDirection::Ascending);
            }
            MenuAction::SortDescending => {
                self.sort_selected(SortDirection::Descending);
            }
            MenuAction::RemoveSort => self.document.remove_sort(),
            MenuAction::MoveRowUp => self.document.move_selected_rows(-1),
            MenuAction::MoveRowDown => self.document.move_selected_rows(1),
            MenuAction::MoveColumnLeft => self.document.move_selected_columns(-1),
            MenuAction::MoveColumnRight => self.document.move_selected_columns(1),
            MenuAction::ToggleFilter => self.document.toggle_filter_bar(),
            _ => {}
        }
    }

    fn sort_selected(&mut self, direction: SortDirection) {
        let Some(column) = self.document.selection().columns.first().copied() else {
            return;
        };
        self.document.sort_rows(column, direction, &self.locale);
    }

    fn copy_selection(&self, ctx: &Context) {
        if let Some(text) = self.document.copy_selection(&self.decimal_separator) {
            ctx.copy_text(text);
        }
    }
}
