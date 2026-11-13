//! Rendering a document window: menu bar, chrome, alerts and banners.

use egui::{Ui, ViewportCommand, ViewportId};
use tabtool_ui::document::{
    DocumentAction, DocumentView, LargeUiAction, alert_message, sync_selection_to_grid,
};
use tabtool_ui::theme::current;

use crate::documents::{CloseDecision, EguiClipboard, WindowRequest, close_decision};
use crate::menu::{Capabilities, MenuAction};

impl crate::documents::WindowState {
    pub(crate) fn render(&mut self, ui: &mut Ui) {
        self.handle_close_request(ui);
        if self.should_close {
            return;
        }
        self.collect_dropped_files(ui.ctx());

        let theme = current(ui.ctx());
        let tree = self.menu_tree();
        if let Some(action) = crate::shortcuts::consume_enabled(ui.ctx(), &tree) {
            self.requests.push(WindowRequest::Menu(action));
        }
        egui::Frame::new()
            .fill(theme.colors.window_background)
            .inner_margin(theme.metrics.window_padding)
            .show(ui, |ui| {
                if let Some(action) = crate::menu::show(ui, &tree) {
                    self.requests.push(WindowRequest::Menu(action));
                }
                ui.separator();
                self.render_chrome(ui);
            });

        self.render_alerts(ui);
        self.render_export(ui);

        // Menu clicks, shortcut presses, alert answers and direct edits are
        // handled by the root viewport's logic pass. A child repaint alone
        // does not wake it.
        if !self.requests.is_empty()
            || self.should_close
            || self.autosave.has_unobserved_change(&self.document)
        {
            ui.ctx().request_repaint_of(ViewportId::ROOT);
        }
    }

    fn render_chrome(&mut self, ui: &mut Ui) {
        let ctx = ui.ctx().clone();
        let mut clipboard = EguiClipboard::new(&ctx);
        let locale = self.locale.clone();
        let decimal_separator = self.decimal_separator.clone();
        let save_error = self.issue;
        let actions = DocumentView {
            document: &mut self.document,
            state: &mut self.grid,
            ui_state: &mut self.chrome,
            storage: &*self.file_ops.storage,
            clipboard: &mut clipboard,
            language: self.language,
            locale: &locale,
            decimal_separator: &decimal_separator,
            show_format_bar: self.show_format_bar,
            save_error,
        }
        .show(ui);

        sync_selection_to_grid(&self.document, &mut self.grid);

        for action in actions {
            match action {
                DocumentAction::Export => self.open_export_panel(),
                DocumentAction::ShowFormatBar => {
                    if !self.show_format_bar {
                        self.app_actions.push(MenuAction::ToggleFormatBar);
                    }
                }
                DocumentAction::Large(LargeUiAction::CancelSave) => self.cancel_save(),
                DocumentAction::Large(LargeUiAction::ReloadStale) => {
                    if let Err(error) = self.document.reload_large(&*self.file_ops.storage) {
                        self.chrome.show_alert(
                            "The file could not be read",
                            alert_message(self.language, error),
                        );
                    }
                }
            }
        }
    }

    fn collect_dropped_files(&mut self, ctx: &egui::Context) {
        let files: Vec<std::path::PathBuf> = ctx.input(|input| {
            input
                .raw
                .dropped_files
                .iter()
                .map(|file| file.path().to_path_buf())
                .collect()
        });
        if !files.is_empty() {
            self.accept_dropped(files);
        }
    }

    fn handle_close_request(&mut self, ui: &Ui) {
        if !ui.ctx().input(|input| input.viewport().close_requested()) {
            return;
        }
        if self.should_close {
            return;
        }
        match close_decision(&self.document) {
            CloseDecision::Close => self.should_close = true,
            CloseDecision::Ask => {
                self.close_prompt = true;
                ui.ctx().send_viewport_cmd(ViewportCommand::CancelClose);
            }
            CloseDecision::Busy => {
                self.busy = true;
                ui.ctx().send_viewport_cmd(ViewportCommand::CancelClose);
            }
        }
    }

    fn menu_tree(&self) -> crate::menu::MenuTree {
        crate::menu::menus(
            self.language,
            &Capabilities::of(&self.document),
            self.show_format_bar,
            self.has_recent,
        )
    }
}
