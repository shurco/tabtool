//! Rendering a document window: menu bar, placeholder, alerts and banners.

use egui::{RichText, Ui, ViewportCommand, ViewportId};
use tabtool_core::CsvWriteIssue;
use tabtool_document::{DocumentError, DocumentPhase};
use tabtool_i18n::tr;
use tabtool_ui::theme::current;
use tabtool_ui::widgets::{BannerKind, banner};

use crate::documents::{CloseDecision, WindowRequest, WindowState, close_decision};
use crate::menu::Capabilities;

impl WindowState {
    /// Renders the window and reacts to its native close request.
    pub(crate) fn render(&mut self, ui: &mut Ui) {
        self.handle_close_request(ui);
        if self.should_close {
            return;
        }

        let theme = current(ui.ctx());
        let tree = self.menu_tree();
        if let Some(action) = crate::shortcuts::consume_enabled(ui.ctx(), &tree) {
            self.requests.push(WindowRequest::Menu(action));
            ui.ctx().request_repaint();
        }
        egui::Frame::new()
            .fill(theme.colors.window_background)
            .inner_margin(theme.metrics.window_padding)
            .show(ui, |ui| {
                if let Some(action) = crate::menu::show(ui, &tree) {
                    self.requests.push(WindowRequest::Menu(action));
                }
                ui.separator();
                self.render_placeholder(ui);
                self.render_issue(ui);
                self.render_saving(ui);
            });

        self.render_alerts(ui);

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

    fn render_placeholder(&mut self, ui: &mut Ui) {
        let theme = current(ui.ctx());
        ui.add_space(theme.metrics.item_spacing);
        ui.heading(self.title());
        if let Some(path) = self.document.file_path() {
            ui.label(
                RichText::new(path.display().to_string())
                    .size(theme.typography.caption)
                    .color(theme.colors.secondary_label),
            );
        }
        ui.add_space(theme.metrics.item_spacing);
        ui.label(
            RichText::new(self.phase_text())
                .size(theme.typography.body)
                .color(theme.colors.label),
        );
        ui.add_space(theme.metrics.window_padding);
        ui.separator();
        ui.add_space(theme.metrics.item_spacing);
        ui.label(
            RichText::new(tr(self.language, "CSV Table"))
                .size(theme.typography.title)
                .color(theme.colors.label),
        );
        ui.label(
            RichText::new(tr(
                self.language,
                "Use the arrow keys to move between cells. Press Return to edit the selected cell.",
            ))
            .size(theme.typography.caption)
            .color(theme.colors.secondary_label),
        );
    }

    fn render_issue(&self, ui: &mut Ui) {
        let Some(error) = self.issue else {
            return;
        };
        banner(
            ui,
            BannerKind::Error,
            tr(self.language, "The file could not be saved"),
            &self.issue_message(error),
            None,
        );
    }

    fn render_saving(&self, ui: &mut Ui) {
        if !self.saving || self.close_after_save {
            return;
        }
        if let Some(progress) = self.document.large_save_progress() {
            let theme = current(ui.ctx());
            ui.label(
                RichText::new(crate::documents::saving_text(self.language, progress))
                    .size(theme.typography.caption)
                    .color(theme.colors.secondary_label),
            );
        }
    }

    fn phase_text(&self) -> String {
        match self.document.phase() {
            DocumentPhase::Loading => tr(self.language, "Loading…").to_owned(),
            DocumentPhase::ReadFailed(_) | DocumentPhase::UnreadableFile => {
                tr(self.language, "The document is not available.").to_owned()
            }
            DocumentPhase::NeedsAttention(issue) => self.write_issue_text(*issue),
            DocumentPhase::Ready => {
                let (rows, columns) = match self.document.table() {
                    Some(table) => (table.row_count(), table.columns.len()),
                    None => match self.document.row_source() {
                        Some(source) => (source.row_count(), source.column_count()),
                        None => (0, 0),
                    },
                };
                format!(
                    "{} · {}",
                    fill_number(tr(self.language, "%lld rows"), rows),
                    fill_number(tr(self.language, "%lld columns"), columns),
                )
            }
        }
    }

    fn write_issue_text(&self, issue: CsvWriteIssue) -> String {
        match issue {
            CsvWriteIssue::AmbiguousOutput => tr(
                self.language,
                "This document can’t be saved with quotes off.",
            )
            .to_owned(),
            CsvWriteIssue::UnencodableText => tr(
                self.language,
                "This document can’t be saved in its current encoding.",
            )
            .to_owned(),
        }
    }

    fn issue_message(&self, error: DocumentError) -> String {
        crate::documents::issue_message(self.language, error)
    }
}

/// Replaces the `%lld` count placeholder.
fn fill_number(template: &str, value: usize) -> String {
    template
        .replace("%lld", &value.to_string())
        .replace("%%", "%")
}

#[cfg(test)]
mod tests {
    use super::fill_number;

    #[test]
    fn count_placeholders_are_replaced() {
        assert_eq!(fill_number("%lld rows", 3), "3 rows");
        assert_eq!(fill_number("%lld%%", 3), "3%");
    }
}
