//! The root viewport's empty state.

use egui::Ui;
use tabtool_i18n::tr;

use crate::app::DesktopApp;
use crate::menu::Capabilities;
use crate::shortcuts;

impl DesktopApp {
    pub(super) fn render_welcome(&mut self, ui: &mut Ui) {
        let theme = tabtool_ui::theme::current(ui.ctx());
        egui::Frame::new()
            .fill(theme.colors.window_background)
            .inner_margin(theme.metrics.window_padding)
            .show(ui, |ui| {
                let tree = crate::menu::menus(
                    self.language,
                    &Capabilities::none(),
                    self.settings.show_format_bar,
                    self.files.has_recent(),
                );
                if let Some(action) = crate::menu::show(ui, &tree) {
                    self.perform(action, ui.ctx());
                }
                let blocked = self.quit.prompt
                    || self.quit.busy
                    || self.quit.issue.is_some()
                    || !self.quit.saving.is_empty();
                if let Some(action) = shortcuts::consume_enabled(ui.ctx(), &tree, blocked) {
                    self.perform(action, ui.ctx());
                }
                ui.separator();
                ui.add_space(theme.metrics.window_padding * 4.0);
                ui.vertical_centered(|ui| {
                    ui.heading(tr(self.language, "No Documents"));
                    ui.label(tr(self.language, "Create a document or import a CSV file."));
                    ui.add_space(theme.metrics.item_spacing);
                    if tabtool_ui::widgets::primary_button(ui, tr(self.language, "New Document"))
                        .clicked()
                    {
                        self.new_document();
                    }
                });
            });
    }
}
