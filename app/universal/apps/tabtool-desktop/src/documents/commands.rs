//! Menu dispatch for a document window.

use std::collections::BTreeSet;

use egui::Context;
use tabtool_core::SortDirection;
use tabtool_document::{GridSelection, RevertDecision, plan_revert};

use crate::documents::{CloseChoice, WindowRequest, WindowState};
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
                WindowRequest::CancelSave => self.cancel_save(),
            }
        }
    }

    fn handle_menu(&mut self, action: MenuAction, ctx: &Context) {
        if action.is_app_action() {
            self.app_actions.push(action);
            return;
        }
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
            MenuAction::RevertToSaved => match plan_revert(&self.document) {
                RevertDecision::Revert => self.revert_now(),
                RevertDecision::NeedsConfirmation => self.revert_prompt = true,
                RevertDecision::Unavailable => {}
            },
            // Export, the inline rename editor and clipboard paste belong to
            // the document chrome and grid tasks; the shell records them so
            // those views can consume the same command.
            MenuAction::Export | MenuAction::RenameColumn | MenuAction::Paste => {
                self.pending_actions.push(action);
            }
            command => self.perform_document_command(command, ctx),
        }
    }

    fn revert_now(&mut self) {
        self.revert_prompt = false;
        if let Err(error) = self.document.revert_to_saved(&*self.storage) {
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
        if let Some(text) = self
            .document
            .copy_selection(&self.locale, &self.decimal_separator)
        {
            ctx.copy_text(text);
        }
    }
}
