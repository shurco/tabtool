//! Drawing the document panels: content, format bar, banners and status.

use egui::{Align, Frame, Layout, RichText, Ui};
use tabtool_document::{Document, DocumentPhase, LargeStatus};
use tabtool_i18n::{Language, tr};

use crate::grid::{GridAction, GridMenuOptions, GridOptions, GridPresenter, GridView, SortSpec};
use crate::theme::current;
use crate::widgets::{AlertAction, ButtonKind, modal};

use super::progress::{self, saving_text};
use super::{
    DocumentAction, DocumentView, LargeUiAction, banners, error_overlay, format_bar, status_line,
};

impl DocumentView<'_> {
    pub(super) fn show_content(&mut self, ui: &mut Ui) -> Vec<GridAction> {
        let phase = *self.document.phase();
        match phase {
            DocumentPhase::ReadFailed(_) | DocumentPhase::UnreadableFile => {
                let can_detect = self.document.file_path().is_some() && !self.is_busy();
                if error_overlay::show(ui, self.language, &phase, can_detect) {
                    self.detect_format();
                }
                Vec::new()
            }
            DocumentPhase::Loading => {
                centered_text(ui, tr(self.language, "Loading…"));
                Vec::new()
            }
            DocumentPhase::Ready | DocumentPhase::NeedsAttention(_) => {
                if self.document.has_content() {
                    self.show_grid(ui)
                } else {
                    let status_fraction = match self.document.large_status() {
                        LargeStatus::Saving(fraction) => Some(fraction),
                        _ => None,
                    };
                    if let Some(fraction) = self.document.large_save_progress().or(status_fraction)
                    {
                        centered_progress(ui, self.language, fraction);
                    } else {
                        centered_text(ui, tr(self.language, "Loading…"));
                    }
                    Vec::new()
                }
            }
        }
    }

    pub(super) fn show_grid(&mut self, ui: &mut Ui) -> Vec<GridAction> {
        let Some(source) = self.document.row_source() else {
            return Vec::new();
        };
        let options = grid_options(
            self.document,
            self.language,
            self.row_height,
            self.alternating_rows,
            self.shows_row_numbers,
        );
        let visible = self.document.row_filter().visible_rows.as_deref();
        let presenter = GridPresenter::new(source, visible);
        GridView {
            source: &presenter,
            state: &mut *self.state,
            options,
        }
        .show(ui)
    }

    pub(super) fn show_status(&mut self, ui: &mut Ui, actions: &mut Vec<DocumentAction>) {
        let status = self.document.large_status();
        let show = self.document.row_source().is_some() || matches!(status, LargeStatus::Saving(_));
        if !show {
            return;
        }
        let summary = status_line::document_summary(self.document, self.language, self.locale)
            .unwrap_or_else(|| match status {
                LargeStatus::Saving(fraction) => saving_text(self.language, fraction),
                _ => String::new(),
            });
        let panel = egui::Panel::bottom("document-status")
            .frame(Frame::NONE)
            .show_separator_line(false);
        if panel
            .show(ui, |ui| {
                status_line::show(ui, self.language, &summary, status)
            })
            .inner
        {
            actions.push(DocumentAction::Large(LargeUiAction::CancelSave));
        }
    }

    pub(super) fn show_filter_bar(&mut self, ui: &mut Ui) {
        if !self.document.can_filter() || !self.document.row_filter().is_bar_visible {
            return;
        }
        let theme = current(ui.ctx());
        egui::Panel::top("document-filter-bar")
            .frame(
                Frame::new()
                    .fill(theme.colors.header_background)
                    .inner_margin(egui::Margin::symmetric(10, 6)),
            )
            .show_separator_line(true)
            .show(ui, |ui| {
                if let Some(action) = crate::filter::show(ui, self.document, self.language) {
                    self.apply_filter_action(action);
                }
            });
    }

    pub(super) fn show_format_bar(&mut self, ui: &mut Ui) {
        if !self.show_format_bar {
            return;
        }
        let busy = self.is_busy();
        let detect_available = self.document.file_path().is_some();
        let action = egui::Panel::bottom("document-format-bar")
            .frame(Frame::NONE)
            .show_separator_line(false)
            .show(ui, |ui| {
                format_bar::show(ui, self.document, self.language, busy, detect_available)
            })
            .inner;
        if let Some(action) = action {
            self.apply_format_action(action);
        }
    }

    pub(super) fn show_alert(&mut self, ui: &mut Ui) {
        let Some(alert) = self.ui_state.alert.clone() else {
            return;
        };
        let dismissed = egui::Panel::bottom("document-format-alert")
            .frame(Frame::NONE)
            .show_separator_line(false)
            .show(ui, |ui| {
                egui::Frame::new()
                    .inner_margin(egui::Margin::symmetric(12, 6))
                    .show(ui, |ui| {
                        crate::widgets::banner(
                            ui,
                            crate::widgets::BannerKind::Error,
                            tr(self.language, alert.title),
                            &alert.message,
                            Some(tr(self.language, "OK")),
                        )
                        .action_clicked
                    })
                    .inner
            })
            .inner;
        if dismissed {
            self.ui_state.alert = None;
        }
    }

    pub(super) fn show_notice(&mut self, ui: &mut Ui) {
        let Some(notice) = self.ui_state.notice else {
            return;
        };
        let dismissed = egui::Panel::bottom("document-notice")
            .frame(Frame::NONE)
            .show_separator_line(false)
            .show(ui, |ui| {
                egui::Frame::new()
                    .inner_margin(egui::Margin::symmetric(12, 6))
                    .show(ui, |ui| banners::notice(ui, self.language, notice))
                    .inner
            })
            .inner;
        if dismissed {
            self.ui_state.notice = None;
        }
    }

    pub(super) fn show_needs_attention(&mut self, ui: &mut Ui, actions: &mut Vec<DocumentAction>) {
        let Some(issue) = self.document.write_issue() else {
            return;
        };
        let can_undo = self.document.can_undo();
        let response = egui::Panel::bottom("document-needs-attention")
            .frame(Frame::NONE)
            .show_separator_line(false)
            .show(ui, |ui| {
                banners::needs_attention(ui, self.language, issue, can_undo)
            })
            .inner;
        if response.undo {
            self.document.undo();
        }
        if response.show_format_bar {
            actions.push(DocumentAction::ShowFormatBar);
        }
    }

    pub(super) fn show_stale(&mut self, ui: &mut Ui) -> bool {
        let LargeStatus::Stale { appended } = self.document.large_status() else {
            return false;
        };
        egui::Panel::bottom("document-stale")
            .frame(Frame::NONE)
            .show_separator_line(false)
            .show(ui, |ui| banners::stale(ui, self.language, appended))
            .inner
    }

    pub(super) fn show_large_banner(&mut self, ui: &mut Ui) {
        if !self.document.is_large_mode() || !self.document.has_content() {
            return;
        }
        let status = self.document.large_status();
        let rows = self
            .document
            .row_source()
            .map_or(0, |source| source.row_count());
        egui::Panel::bottom("document-large-banner")
            .frame(Frame::NONE)
            .show_separator_line(false)
            .show(ui, |ui| {
                banners::large_file(ui, self.language, status, rows, self.locale);
            });
    }

    pub(super) fn show_save_error(&mut self, ui: &mut Ui) {
        let Some(error) = self.save_error else {
            return;
        };
        if self.document.write_issue().is_some() {
            return;
        }
        egui::Panel::bottom("document-save-error")
            .frame(Frame::NONE)
            .show_separator_line(false)
            .show(ui, |ui| {
                egui::Frame::new()
                    .inner_margin(egui::Margin::symmetric(12, 6))
                    .show(ui, |ui| banners::save_error(ui, self.language, error));
            });
    }

    pub(super) fn show_reparse_modal(&mut self, ui: &mut Ui) {
        let Some(configuration) = self.ui_state.pending_reparse.clone() else {
            return;
        };
        let actions = [
            AlertAction::new(tr(self.language, "Re-read"), ButtonKind::Destructive),
            AlertAction::new(tr(self.language, "Cancel"), ButtonKind::Bordered),
        ];
        let chosen = modal(
            ui,
            "document-reparse",
            tr(self.language, "Re-read the file and discard unsaved edits?"),
            tr(
                self.language,
                "The table will be replaced with the last saved version from disk. The file itself is not changed.",
            ),
            &actions,
        );
        let Some(index) = chosen else {
            return;
        };
        self.ui_state.pending_reparse = None;
        if index == 0 {
            self.apply_configuration(&configuration, true);
        }
    }
}

fn grid_options(
    document: &Document,
    language: Language,
    row_height: f32,
    alternating_rows: bool,
    shows_row_numbers: bool,
) -> GridOptions {
    let sort = document.active_sort().and_then(|sort| {
        document.row_source().and_then(|source| {
            source
                .column_ids()
                .iter()
                .position(|column| *column == sort.column_id)
                .map(|column| SortSpec {
                    column,
                    direction: sort.direction,
                })
        })
    });
    let header_offset = if document.is_large_mode() && document.configuration().first_row_as_header
    {
        1
    } else {
        0
    };
    GridOptions {
        row_height,
        alternating_rows,
        shows_row_numbers,
        decimal_mark: document.configuration().decimal_mark.clone(),
        editable: document.can_edit_cells(),
        sort,
        language,
        header_offset,
        menu: GridMenuOptions::of(document),
        ..GridOptions::default()
    }
}

fn centered_text(ui: &mut Ui, text: &str) {
    let theme = current(ui.ctx());
    ui.with_layout(Layout::top_down(Align::Center), |ui| {
        ui.add_space(theme.metrics.window_padding * 3.0);
        ui.label(
            RichText::new(text)
                .size(theme.typography.body)
                .color(theme.colors.secondary_label),
        );
    });
}

fn centered_progress(ui: &mut Ui, language: Language, fraction: f64) {
    let theme = current(ui.ctx());
    ui.with_layout(Layout::top_down(Align::Center), |ui| {
        ui.add_space(theme.metrics.window_padding * 3.0);
        ui.label(
            RichText::new(saving_text(language, fraction))
                .size(theme.typography.body)
                .color(theme.colors.secondary_label),
        );
        progress::progress_bar(ui, fraction, 160.0);
    });
}
