//! The library screen: toolbar, rows, empty state, banners and prompts.

use egui::{RichText, ScrollArea, Ui};
use tabtool_i18n::tr;

use super::{LibraryResponse, LibraryView, format_modified, format_size};
use crate::fonts::heading_font;
use crate::mobile::{ListRow, TouchTextInput, TouchTextInputOutcome, TouchTextInputRequest};
use crate::theme::current;
use crate::widgets::{AlertAction, BannerKind, ButtonKind, banner, modal};

mod toolbar;

impl LibraryView {
    /// Shows the library and reports what the app should do next.
    pub fn show(&mut self, ui: &mut Ui, text_input: &mut dyn TouchTextInput) -> LibraryResponse {
        if !self.loaded {
            self.refresh();
        }
        self.poll_error();
        self.poll_import();
        self.poll_rename(text_input);

        let mut response = LibraryResponse::default();
        self.toolbar(ui, &mut response);
        self.show_notice(ui);
        if self.documents.is_empty() {
            self.show_empty(ui);
        } else {
            self.show_list(ui, &mut response);
        }
        self.open_rename(text_input);
        self.show_delete_confirmation(ui);
        if response.open.is_none() {
            response.open = self.pending_open.take();
        } else {
            self.pending_open = None;
        }
        response
    }

    fn show_notice(&mut self, ui: &mut Ui) {
        let Some(error) = self.notice.as_ref() else {
            return;
        };
        let title = tr(self.language, error.message_key);
        let shown = banner(
            ui,
            BannerKind::Error,
            title,
            "",
            Some(tr(self.language, "OK")),
        );
        if shown.action_clicked {
            self.notice = None;
        }
    }

    fn show_empty(&self, ui: &mut Ui) {
        let theme = current(ui.ctx());
        let title = tr(self.language, "No Documents");
        let message = tr(self.language, "Create a document or import a CSV file.");
        let available = ui.available_height();
        ui.add_space((available / 2.0 - 40.0).max(0.0));
        ui.vertical_centered(|ui| {
            ui.label(
                RichText::new(title)
                    .font(heading_font(self.metrics.title))
                    .color(theme.colors.label),
            );
            ui.add_space(6.0);
            ui.label(
                RichText::new(message)
                    .size(self.metrics.caption)
                    .color(theme.colors.secondary_label),
            );
        });
    }

    fn show_list(&mut self, ui: &mut Ui, response: &mut LibraryResponse) {
        let mut opened = None;
        let mut delete_request = None;
        let mut rename_request = None;
        ScrollArea::vertical()
            .id_salt("library-list")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for document in &self.documents {
                    let subtitle = self.subtitle(document);
                    let row = ListRow::new(document.title())
                        .subtitle(&subtitle)
                        .chevron(true)
                        .show(ui, self.metrics, self.language);
                    if row.activated {
                        opened = Some(document.path.clone());
                    }
                    if row.delete_requested {
                        delete_request = Some(document.path.clone());
                    }
                    if row.rename_requested {
                        rename_request = Some((document.path.clone(), row.response.rect));
                    }
                }
            });
        if let Some(path) = opened {
            response.open = Some(path);
        }
        if let Some(path) = delete_request {
            self.pending_delete = Some(path);
        }
        if let Some((path, rect)) = rename_request {
            self.rename_request = Some((path, rect));
        }
    }

    fn show_delete_confirmation(&mut self, ui: &mut Ui) {
        let Some(path) = self.pending_delete.clone() else {
            return;
        };
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .to_owned();
        let actions = [
            AlertAction::new(tr(self.language, "Delete"), ButtonKind::Destructive),
            AlertAction::new(tr(self.language, "Cancel"), ButtonKind::Bordered),
        ];
        let pressed = modal(
            ui,
            "library-delete",
            tr(self.language, "Delete Document?"),
            &name,
            &actions,
        );
        let Some(index) = pressed else {
            return;
        };
        self.pending_delete = None;
        if index == 0
            && let Err(error) = self.delete_document(&path)
        {
            self.notice = Some(error);
        }
    }

    fn open_rename(&mut self, text_input: &mut dyn TouchTextInput) {
        let Some((path, rect)) = self.rename_request.take() else {
            return;
        };
        let text = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .to_owned();
        self.renaming = Some(path);
        text_input.open(TouchTextInputRequest {
            rect,
            text,
            multiline: false,
        });
    }

    fn poll_rename(&mut self, text_input: &mut dyn TouchTextInput) {
        let Some(path) = self.renaming.clone() else {
            return;
        };
        match text_input.poll() {
            Some(TouchTextInputOutcome::Committed(name)) => {
                self.renaming = None;
                if let Err(error) = self.rename_document(&path, &name) {
                    self.notice = Some(error);
                }
            }
            Some(TouchTextInputOutcome::Cancelled) => self.renaming = None,
            None if !text_input.is_active() => self.renaming = None,
            None => {}
        }
    }

    fn poll_error(&mut self) {
        if self.notice.is_none() {
            self.notice = self.files.take_error();
        }
    }

    fn poll_import(&mut self) {
        if !self.importing {
            return;
        }
        if self.files.import_in_progress() {
            return;
        }
        self.importing = false;
        if let Some(path) = self.files.import_from_picker() {
            self.pending_open = self.accept_import(path);
        }
    }

    fn subtitle(&self, document: &super::DocumentEntry) -> String {
        format!(
            "{} \u{00B7} {}",
            format_size(document.size(), &self.locale),
            format_modified(document.modified(), &self.locale, self.clock)
        )
    }
}
