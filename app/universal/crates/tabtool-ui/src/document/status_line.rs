//! The bottom status line: counts, format summary, filter count and save
//! progress with Cancel.

use egui::{Align, Frame, Layout, Margin, RichText, Stroke, Ui};
use tabtool_core::{CsvConfiguration, Locale};
use tabtool_document::{Document, LargeStatus};
use tabtool_i18n::{Language, tr};

use super::progress::{grouped_count, progress_bar, saving_text};
use crate::theme::current;
use crate::widgets::{ButtonKind, PushButton};

/// The "rows × columns · encoding · separator" summary.
///
/// While a large file indexes or saves, the progress line replaces the counts,
/// matching the macOS status bar.
pub fn status_summary(
    language: Language,
    rows: usize,
    columns: usize,
    configuration: &CsvConfiguration,
    visible_rows: Option<usize>,
    status: LargeStatus,
    locale: &Locale,
) -> String {
    match status {
        LargeStatus::Indexing(fraction) => {
            return super::progress::indexing_text(language, fraction, rows, locale);
        }
        LargeStatus::Saving(fraction) => return saving_text(language, fraction),
        LargeStatus::None | LargeStatus::Ready | LargeStatus::Stale { .. } => {}
    }
    let rows_text = rows_text(language, rows, visible_rows, locale);
    let columns_text = columns_text(language, columns, locale);
    format!(
        "{rows_text} × {columns_text} · {} · {}",
        tr(language, configuration.encoding.display_name()),
        separator_name(language, &configuration.column_separator)
    )
}

/// The summary for a loaded document, or `None` without a row source.
pub fn document_summary(
    document: &Document,
    language: Language,
    locale: &Locale,
) -> Option<String> {
    let source = document.row_source()?;
    Some(status_summary(
        language,
        source.row_count(),
        source.column_count(),
        document.configuration(),
        document.row_filter().visible_rows.as_ref().map(Vec::len),
        document.large_status(),
        locale,
    ))
}

/// Draws the status line.
///
/// A running large save adds a progress bar and a Cancel button. Returns
/// whether Cancel was pressed.
pub fn show(ui: &mut Ui, language: Language, summary: &str, status: LargeStatus) -> bool {
    let theme = current(ui.ctx());
    let colors = theme.colors;
    let mut cancel = false;
    let frame = Frame::new()
        .fill(colors.header_background)
        .inner_margin(Margin::symmetric(
            theme.metrics.status_padding_x as i8,
            theme.metrics.status_padding_y as i8,
        ))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.set_height(theme.metrics.status_line_height - 2.0 * theme.metrics.status_padding_y);
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(summary)
                        .size(theme.typography.caption)
                        .color(colors.secondary_label),
                );
                if let LargeStatus::Saving(fraction) = status {
                    progress_bar(ui, fraction, 64.0);
                }
                if matches!(status, LargeStatus::Saving(_)) {
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        cancel = PushButton::new(tr(language, "Cancel"), ButtonKind::Bordered)
                            .show(ui)
                            .clicked();
                    });
                }
            });
        });

    let rect = frame.response.rect;
    ui.painter().hline(
        rect.x_range(),
        rect.top(),
        Stroke::new(1.0, colors.separator),
    );
    cancel
}

fn rows_text(language: Language, rows: usize, visible: Option<usize>, locale: &Locale) -> String {
    match visible {
        Some(visible) => fill_pair(
            tr(language, "%lld of %lld rows"),
            &grouped_count(visible, locale),
            &grouped_count(rows, locale),
        ),
        None if rows == 1 => tr(language, "1 row").to_owned(),
        None => fill_one(tr(language, "%lld rows"), &grouped_count(rows, locale)),
    }
}

fn columns_text(language: Language, columns: usize, locale: &Locale) -> String {
    if columns == 1 {
        tr(language, "1 column").to_owned()
    } else {
        fill_one(
            tr(language, "%lld columns"),
            &grouped_count(columns, locale),
        )
    }
}

/// Replaces a `%lld` placeholder with `value`.
fn fill_one(template: &str, value: &str) -> String {
    template.replace("%lld", value)
}

/// Replaces the positional or plain two-count placeholders.
fn fill_pair(template: &str, first: &str, second: &str) -> String {
    let text = template.replace("%1$lld", first).replace("%2$lld", second);
    if let Some(index) = text.find("%lld") {
        let rest = text[index + 4..].replacen("%lld", second, 1);
        format!("{}{}{}", &text[..index], first, rest)
    } else {
        text
    }
}

/// The localized name of a separator character for the status summary.
pub fn separator_name(language: Language, separator: &str) -> String {
    match separator {
        "," => tr(language, "Comma").to_owned(),
        ";" => tr(language, "Semicolon").to_owned(),
        "\t" => tr(language, "Tab").to_owned(),
        "|" => tr(language, "Pipe").to_owned(),
        other => other.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::fill_pair;

    #[test]
    fn plain_and_positional_count_placeholders_are_filled() {
        assert_eq!(fill_pair("%lld of %lld rows", "2", "3"), "2 of 3 rows");
        assert_eq!(fill_pair("%1$lld of %2$lld rows", "2", "3"), "2 of 3 rows");
    }
}
