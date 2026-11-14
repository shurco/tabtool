//! The persistent banners: large mode, a stale file, needs attention, a
//! one-shot notice and a failed save.

use egui::{Frame, Margin, RichText, Ui};
use tabtool_core::{CsvWriteIssue, Locale};
use tabtool_document::{DocumentError, DocumentNotice, LargeStatus};
use tabtool_i18n::{Language, tr};

use super::error_overlay::{alert_message, error_message};
use crate::theme::current;
use crate::widgets::{BannerKind, ButtonKind, PushButton, banner};

/// The large-mode explanation, shown for the whole life of the window.
///
/// While the index is being built, the banner also reports the scanned share
/// and the rows known so far.
pub fn large_file(
    ui: &mut Ui,
    language: Language,
    status: LargeStatus,
    rows: usize,
    locale: &Locale,
) {
    let theme = current(ui.ctx());
    Frame::new()
        .fill(theme.colors.banner_info_background)
        .inner_margin(Margin::symmetric(
            12,
            theme.metrics.banner_padding as i8,
        ))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("\u{2139}")
                        .size(theme.typography.title)
                        .color(theme.colors.accent),
                );
                ui.add(
                    egui::Label::new(
                        RichText::new(tr(
                            language,
                            "Large file mode. You can browse and edit cells. Sorting, adding, deleting or moving rows and columns, renaming columns, changing the decimal mark and exporting are not available for files this size.",
                        ))
                        .size(theme.typography.caption)
                        .color(theme.colors.secondary_label),
                    )
                    .wrap(),
                );
            });
            if let LargeStatus::Indexing(fraction) = status {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(super::progress::indexing_text(
                            language,
                            fraction,
                            rows,
                            locale,
                        ))
                        .size(theme.typography.caption)
                        .color(theme.colors.secondary_label),
                    );
                    super::progress::progress_bar(ui, fraction, 96.0);
                });
            }
        });
}

/// The stale-file warning; returns whether Reload was pressed.
pub fn stale(ui: &mut Ui, language: Language, appended: bool) -> bool {
    let theme = current(ui.ctx());
    let mut reload = false;
    Frame::new()
        .fill(theme.colors.banner_warning_background)
        .inner_margin(Margin::symmetric(12, theme.metrics.banner_padding as i8))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("\u{26A0}")
                        .size(theme.typography.title)
                        .color(theme.colors.warning),
                );
                ui.add(
                    egui::Label::new(
                        RichText::new(if appended {
                            tr(language, "New rows were appended. Reload to see them.")
                        } else {
                            tr(language, "The file changed on disk. Reload to continue.")
                        })
                        .font(crate::fonts::heading_font(theme.typography.body))
                        .color(theme.colors.label),
                    )
                    .wrap(),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    reload = PushButton::new(tr(language, "Reload"), ButtonKind::Bordered)
                        .show(ui)
                        .clicked();
                });
            });
        });
    reload
}

/// What the needs-attention banner's buttons asked for.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct NeedsAttentionResponse {
    /// Undo Last Edit was pressed.
    pub undo: bool,
    /// Show Format Controls was pressed.
    pub show_format_bar: bool,
}

/// Explains why the document cannot be saved; offers Undo and format controls.
pub fn needs_attention(
    ui: &mut Ui,
    language: Language,
    issue: CsvWriteIssue,
    can_undo: bool,
) -> NeedsAttentionResponse {
    let theme = current(ui.ctx());
    let (title, detail) = issue_text(language, issue);
    let mut response = NeedsAttentionResponse::default();
    Frame::new()
        .fill(theme.colors.banner_warning_background)
        .inner_margin(Margin::symmetric(12, theme.metrics.banner_padding as i8))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("\u{26A0}")
                        .size(theme.typography.title)
                        .color(theme.colors.warning),
                );
                ui.add(
                    egui::Label::new(
                        RichText::new(title)
                            .font(crate::fonts::heading_font(theme.typography.body))
                            .color(theme.colors.label),
                    )
                    .wrap(),
                );
            });
            ui.add(
                egui::Label::new(
                    RichText::new(detail)
                        .size(theme.typography.caption)
                        .color(theme.colors.secondary_label),
                )
                .wrap(),
            );
            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    response.show_format_bar =
                        PushButton::new(tr(language, "Show Format Controls"), ButtonKind::Bordered)
                            .show(ui)
                            .clicked();
                    response.undo =
                        PushButton::new(tr(language, "Undo Last Edit"), ButtonKind::Bordered)
                            .enabled(can_undo)
                            .show(ui)
                            .clicked();
                });
            });
        });
    response
}

/// Shows a one-shot document notice; returns whether it was dismissed.
pub fn notice(ui: &mut Ui, language: Language, notice: DocumentNotice) -> bool {
    let message = match notice {
        DocumentNotice::CopyLimitExceeded => tr(
            language,
            "Too many cells selected to copy (limit 1,000,000).",
        )
        .to_owned(),
        DocumentNotice::EditRejected(error) => alert_message(language, DocumentError::Csv(error)),
    };
    banner(
        ui,
        BannerKind::Warning,
        tr(language, "Edit Not Applied"),
        &message,
        Some(tr(language, "OK")),
    )
    .action_clicked
}

/// Explains a failed save above the format bar.
pub fn save_error(ui: &mut Ui, language: Language, error: DocumentError) {
    banner(
        ui,
        BannerKind::Error,
        tr(language, "The file could not be saved"),
        &error_message(language, error),
        None,
    );
}

fn issue_text(language: Language, issue: CsvWriteIssue) -> (&'static str, &'static str) {
    match issue {
        CsvWriteIssue::UnencodableText => (
            tr(
                language,
                "This document can’t be saved in its current encoding.",
            ),
            tr(
                language,
                "Some text does not exist in the selected encoding. Undo the edit that added it, or export the table in an encoding that can represent it.",
            ),
        ),
        CsvWriteIssue::AmbiguousOutput => (
            tr(language, "This document can’t be saved with quotes off."),
            tr(
                language,
                "A cell contains the separator or a line break and quotes are off. Undo the edit that added it, or export with quotes on.",
            ),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::issue_text;
    use tabtool_core::CsvWriteIssue;
    use tabtool_i18n::Language;

    #[test]
    fn both_write_issues_have_a_title_and_detail() {
        for issue in [
            CsvWriteIssue::UnencodableText,
            CsvWriteIssue::AmbiguousOutput,
        ] {
            let (title, detail) = issue_text(Language::En, issue);
            assert!(!title.is_empty());
            assert!(!detail.is_empty());
        }
    }
}
