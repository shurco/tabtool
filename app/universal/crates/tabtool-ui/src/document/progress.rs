//! Large-file progress wording and the compact progress bar.

use egui::{CornerRadius, Sense, Ui, Vec2};
use tabtool_i18n::{Language, tr};

use crate::theme::current;

/// The whole-percent value of `fraction`, clamped to 0–100.
pub fn percent(fraction: f64) -> i64 {
    (fraction.clamp(0.0, 1.0) * 100.0) as i64
}

/// The localized "Indexing…" line with the scanned share and known rows.
pub fn indexing_text(
    language: Language,
    fraction: f64,
    rows: usize,
    decimal_separator: &str,
) -> String {
    fill(
        tr(language, "Indexing… %lld%% · %@ rows so far"),
        percent(fraction),
        &grouped_count(rows, decimal_separator),
    )
}

/// The localized "Saving…" line with the written share.
pub fn saving_text(language: Language, fraction: f64) -> String {
    fill_percent(tr(language, "Saving… %lld%%"), percent(fraction))
}

/// Draws a small progress bar filling `fraction` of `width`.
pub fn progress_bar(ui: &mut Ui, fraction: f64, width: f32) {
    let theme = current(ui.ctx());
    let height = 4.0;
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, height), Sense::hover());
    let radius = CornerRadius::same(2);
    ui.painter()
        .rect_filled(rect, radius, theme.colors.separator);
    let filled = (fraction.clamp(0.0, 1.0) as f32) * rect.width();
    if filled > 0.0 {
        let fill = egui::Rect::from_min_size(rect.min, Vec2::new(filled, rect.height()));
        ui.painter().rect_filled(fill, radius, theme.colors.accent);
    }
}

/// Groups a row count for the status line.
///
/// The catalog supplies the sentence; the number uses the interface grouping
/// convention: a comma decimal separator groups with dots, everything else
/// groups with commas. That matches the English, German and Spanish status
/// lines without depending on Apple's number formatter.
pub(crate) fn grouped_count(value: usize, decimal_separator: &str) -> String {
    let text = value.to_string();
    let grouping = if decimal_separator == "," { '.' } else { ',' };
    let mut grouped = String::with_capacity(text.len() + text.len() / 3);
    for (index, character) in text.chars().enumerate() {
        if index > 0 && (text.len() - index).is_multiple_of(3) {
            grouped.push(grouping);
        }
        grouped.push(character);
    }
    grouped
}

fn fill_percent(template: &str, percent: i64) -> String {
    template
        .replace("%lld", &percent.to_string())
        .replace("%%", "%")
}

fn fill(template: &str, percent: i64, rows: &str) -> String {
    template
        .replace("%lld", &percent.to_string())
        .replace("%@", rows)
        .replace("%%", "%")
}

#[cfg(test)]
mod tests {
    use super::{grouped_count, percent};

    #[test]
    fn counts_group_by_threes() {
        assert_eq!(grouped_count(0, "."), "0");
        assert_eq!(grouped_count(999, "."), "999");
        assert_eq!(grouped_count(1_000, "."), "1,000");
        assert_eq!(grouped_count(1_234_567, "."), "1,234,567");
        assert_eq!(grouped_count(1_234_567, ","), "1.234.567");
    }

    #[test]
    fn percent_rounds_and_clamps() {
        assert_eq!(percent(-1.0), 0);
        assert_eq!(percent(0.004), 0);
        assert_eq!(percent(0.125), 12);
        assert_eq!(percent(1.5), 100);
    }
}
