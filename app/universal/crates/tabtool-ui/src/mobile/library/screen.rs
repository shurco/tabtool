//! The library screen: toolbar, rows, empty state, banners and prompts.

use egui::{Align2, Button, RichText, Role, ScrollArea, Sense, Ui, Vec2, WidgetInfo, pos2};
use tabtool_i18n::tr;

use super::{LibraryResponse, LibraryView, format_modified, format_size};
use crate::fonts::heading_font;
use crate::icons::Icon;
use crate::mobile::{
    ListRow, TouchCell, TouchTextInput, TouchTextInputOutcome, TouchTextInputRequest, bar_fill,
    hairline,
};
use crate::theme::current;
use crate::widgets::{AlertAction, BannerKind, ButtonKind, banner, modal};

impl LibraryView<'_> {
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

    fn toolbar(&mut self, ui: &mut Ui, response: &mut LibraryResponse) {
        let width = ui.available_width();
        let (rect, _) = ui.allocate_exact_size(
            Vec2::new(width, self.metrics.toolbar_height),
            Sense::hover(),
        );
        ui.painter().rect_filled(rect, 0.0, bar_fill(ui));
        hairline(ui, rect, rect.bottom());

        let cell = self.metrics.control_height;
        let inset = 4.0;
        let settings_rect = egui::Rect::from_min_size(
            pos2(rect.left() + inset, rect.top()),
            Vec2::new(cell, rect.height()),
        );
        let settings = TouchCell::icon(Icon::Settings, tr(self.language, "Settings")).show(
            ui,
            settings_rect,
            "library-settings",
        );
        if settings.clicked() {
            response.settings = true;
        }

        let plus_rect = egui::Rect::from_min_size(
            pos2(rect.right() - inset - cell, rect.top()),
            Vec2::new(cell, rect.height()),
        );
        let plus = TouchCell::icon(Icon::Plus, tr(self.language, "Add Document")).show(
            ui,
            plus_rect,
            "library-add",
        );
        let overflow_rect = egui::Rect::from_min_size(
            pos2(plus_rect.left() - cell, rect.top()),
            Vec2::new(cell, rect.height()),
        );
        let overflow = TouchCell::icon(Icon::Overflow, tr(self.language, "More")).show(
            ui,
            overflow_rect,
            "library-overflow",
        );

        let center = egui::Rect::from_min_max(
            pos2(settings_rect.right() + 8.0, rect.top()),
            pos2(
                (overflow_rect.left() - 8.0).max(settings_rect.right() + 8.0),
                rect.bottom(),
            ),
        );
        let title = tr(self.language, "Documents");
        let title_response =
            ui.interact(center, ui.scope_id().with("library-title"), Sense::hover());
        title_response.widget_info(|| WidgetInfo::labeled(Role::Label, true, title));
        let theme = current(ui.ctx());
        ui.painter().text(
            center.center(),
            Align2::CENTER_CENTER,
            title,
            heading_font(self.metrics.title),
            theme.colors.label,
        );

        egui::Popup::menu(&plus).show(|ui| {
            ui.set_min_width(180.0);
            if menu_item(ui, self.metrics, tr(self.language, "New Document")) {
                match self.create_document() {
                    Ok(path) => response.open = Some(path),
                    Err(error) => self.notice = Some(error),
                }
                ui.close();
            }
            if menu_item(ui, self.metrics, tr(self.language, "Import…")) {
                if let Some(path) = self.import_document() {
                    response.open = Some(path);
                }
                ui.close();
            }
            if menu_item(ui, self.metrics, tr(self.language, "Settings")) {
                response.settings = true;
                ui.close();
            }
        });

        egui::Popup::menu(&overflow).show(|ui| {
            ui.set_min_width(180.0);
            if menu_item(ui, self.metrics, tr(self.language, "Settings")) {
                response.settings = true;
                ui.close();
            }
            if menu_item(ui, self.metrics, tr(self.language, "About")) {
                response.about = true;
                ui.close();
            }
        });
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
            self.notice = self.errors.take_error();
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

fn menu_item(ui: &mut Ui, metrics: crate::mobile::MobileMetrics, label: &str) -> bool {
    ui.add(
        Button::new(RichText::new(label).size(metrics.body))
            .min_size(Vec2::new(180.0, metrics.control_height)),
    )
    .clicked()
}
