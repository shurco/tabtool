//! Large-file progress wording and the compact progress bar.

use egui::{CornerRadius, Role, Sense, Ui, Vec2, WidgetInfo};
use tabtool_core::Locale;
use tabtool_i18n::{Language, tr};

use crate::theme::current;

/// The whole-percent value of `fraction`, clamped to 0–100.
pub fn percent(fraction: f64) -> i64 {
    (fraction.clamp(0.0, 1.0) * 100.0) as i64
}

/// The localized "Indexing…" line with the scanned share and known rows.
pub fn indexing_text(language: Language, fraction: f64, rows: usize, locale: &Locale) -> String {
    fill(
        tr(language, "Indexing… %lld%% · %@ rows so far"),
        percent(fraction),
        &grouped_count(rows, locale),
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
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, height), Sense::hover());
    response.widget_info(|| WidgetInfo {
        role: Role::ProgressIndicator,
        enabled: true,
        label: None,
        current_text_value: None,
        prev_text_value: None,
        selected: None,
        value: Some(fraction.clamp(0.0, 1.0)),
        text_selection: None,
        hint_text: None,
    });
    let radius = CornerRadius::same(2);
    ui.painter()
        .rect_filled(rect, radius, theme.colors.separator);
    let filled = (fraction.clamp(0.0, 1.0) as f32) * rect.width();
    if filled > 0.0 {
        let fill = egui::Rect::from_min_size(rect.min, Vec2::new(filled, rect.height()));
        ui.painter().rect_filled(fill, radius, theme.colors.accent);
    }
}

/// The grouping separator the locale uses for digit groups.
///
/// The app does not link an ICU number formatter yet, so the supported
/// interface languages are mapped explicitly: French narrows the space, the
/// Italian, German and Spanish forms use a dot, and the remaining forms use a
/// comma. Polish, Belarusian and Ukrainian use a no-break space.
pub(crate) fn grouping_separator(locale: &Locale) -> &'static str {
    match locale.id.language.as_str() {
        "fr" => "\u{202F}",
        "pl" | "be" | "uk" => "\u{00A0}",
        "de" | "es" | "it" => ".",
        _ => ",",
    }
}

/// Groups a row count for the status line and the indexing banner.
pub(crate) fn grouped_count(value: usize, locale: &Locale) -> String {
    let text = value.to_string();
    let grouping = grouping_separator(locale);
    let mut grouped = String::with_capacity(text.len() + text.len() / 3);
    for (index, character) in text.chars().enumerate() {
        if index > 0 && (text.len() - index).is_multiple_of(3) {
            grouped.push_str(grouping);
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
    use tabtool_core::Locale;

    use super::{grouped_count, grouping_separator, percent};

    fn locale(code: &str) -> Locale {
        Locale::try_from_str(code).expect("a valid test locale")
    }

    #[test]
    fn counts_group_by_threes() {
        assert_eq!(grouped_count(0, &locale("en")), "0");
        assert_eq!(grouped_count(999, &locale("en")), "999");
        assert_eq!(grouped_count(1_000, &locale("en")), "1,000");
        assert_eq!(grouped_count(1_234_567, &locale("en")), "1,234,567");
        assert_eq!(grouped_count(1_234_567, &locale("de")), "1.234.567");
        assert_eq!(grouped_count(1_234_567, &locale("it")), "1.234.567");
        assert_eq!(
            grouped_count(1_234_567, &locale("fr")),
            "1\u{202F}234\u{202F}567"
        );
        assert_eq!(
            grouped_count(1_234_567, &locale("pl")),
            "1\u{00A0}234\u{00A0}567"
        );
    }

    #[test]
    fn grouping_separator_follows_the_locale() {
        assert_eq!(grouping_separator(&locale("en")), ",");
        assert_eq!(grouping_separator(&locale("de")), ".");
        assert_eq!(grouping_separator(&locale("it")), ".");
        assert_eq!(grouping_separator(&locale("fr")), "\u{202F}");
        assert_eq!(grouping_separator(&locale("uk")), "\u{00A0}");
    }

    #[test]
    fn percent_rounds_and_clamps() {
        assert_eq!(percent(-1.0), 0);
        assert_eq!(percent(0.004), 0);
        assert_eq!(percent(0.125), 12);
        assert_eq!(percent(1.5), 100);
    }
}
