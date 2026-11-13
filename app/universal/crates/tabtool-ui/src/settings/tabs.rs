//! The four Settings tab forms.

use egui::{Grid, RichText, Ui};
use tabtool_core::Encoding;
use tabtool_i18n::{Language, tr};
use tabtool_platform::{LARGE_FILE_THRESHOLDS_MIB, QuoteStyle, ROW_HEIGHTS, Settings};

use crate::format_options::{DECIMALS, SEPARATORS};
use crate::theme::current;
use crate::widgets::{Picker, toggle};

use super::options::{
    decimal_index, default_encoding_index, default_threshold_index, separator_index,
    threshold_label,
};

/// Settings > General: language and the format bar.
pub(super) fn general(
    ui: &mut Ui,
    settings: &mut Settings,
    language: Language,
    changed: &mut bool,
) {
    form_grid(ui, "settings-general", |ui| {
        ui.label(tr(language, "Language"));
        let names: Vec<&str> = Language::ALL
            .iter()
            .map(|candidate| {
                candidate
                    .native_name()
                    .unwrap_or_else(|| tr(language, "System"))
            })
            .collect();
        let selected = Language::ALL
            .iter()
            .position(|candidate| *candidate == settings.app_language)
            .unwrap_or(0);
        if let Some(index) = Picker::new(
            (ui.ctx().viewport_id(), "settings-language"),
            tr(language, "Language"),
            names[selected],
            &names,
        )
        .width(210.0)
        .show(ui)
        {
            settings.app_language = Language::ALL[index];
            *changed = true;
        }
        ui.end_row();

        ui.label("");
        if toggle(
            ui,
            tr(language, "Show Format Bar"),
            &mut settings.show_format_bar,
        )
        .changed()
        {
            *changed = true;
        }
        ui.end_row();
    });
}

/// Settings > New Document: the format of an untitled document.
pub(super) fn new_document(
    ui: &mut Ui,
    settings: &mut Settings,
    language: Language,
    changed: &mut bool,
) {
    form_grid(ui, "settings-new-document", |ui| {
        ui.label(tr(language, "Encoding"));
        let names: Vec<&str> = Encoding::ALL
            .iter()
            .map(|encoding| tr(language, encoding.display_name()))
            .collect();
        let selected = Encoding::ALL
            .iter()
            .position(|encoding| *encoding == settings.new_document_encoding)
            .unwrap_or(default_encoding_index());
        if let Some(index) = Picker::new(
            (ui.ctx().viewport_id(), "settings-encoding"),
            tr(language, "Encoding"),
            names[selected],
            &names,
        )
        .width(210.0)
        .show(ui)
        {
            settings.new_document_encoding = Encoding::ALL[index];
            *changed = true;
        }
        ui.end_row();

        ui.label(tr(language, "Separator"));
        let labels: Vec<&str> = SEPARATORS
            .iter()
            .map(|(_, key)| tr(language, key))
            .collect();
        let selected = separator_index(&settings.new_document_separator);
        if let Some(index) = Picker::new(
            (ui.ctx().viewport_id(), "settings-separator"),
            tr(language, "Separator"),
            labels[selected],
            &labels,
        )
        .width(210.0)
        .show(ui)
        {
            settings.new_document_separator = SEPARATORS[index].0.to_owned();
            *changed = true;
        }
        ui.end_row();

        ui.label(tr(language, "Quote"));
        let labels = [
            tr(language, "Doubled"),
            tr(language, "Backslash"),
            tr(language, "None"),
        ];
        let selected = QuoteStyle::ALL
            .iter()
            .position(|style| *style == settings.new_document_quote_style)
            .unwrap_or(0);
        if let Some(index) = Picker::new(
            (ui.ctx().viewport_id(), "settings-quote"),
            tr(language, "Quote"),
            labels[selected],
            &labels,
        )
        .width(210.0)
        .show(ui)
        {
            settings.new_document_quote_style = QuoteStyle::ALL[index];
            *changed = true;
        }
        ui.end_row();

        ui.label(tr(language, "Decimal Mark"));
        let labels: Vec<&str> = DECIMALS.iter().map(|(_, key)| tr(language, key)).collect();
        let selected = decimal_index(&settings.new_document_decimal_mark);
        if let Some(index) = Picker::new(
            (ui.ctx().viewport_id(), "settings-decimal"),
            tr(language, "Decimal Mark"),
            labels[selected],
            &labels,
        )
        .width(210.0)
        .show(ui)
        {
            settings.new_document_decimal_mark = DECIMALS[index].0.to_owned();
            *changed = true;
        }
        ui.end_row();

        ui.label("");
        if toggle(
            ui,
            tr(language, "First Row as Header"),
            &mut settings.new_document_first_row_as_header,
        )
        .changed()
        {
            *changed = true;
        }
        ui.end_row();
    });
    caption(
        ui,
        language,
        "These apply to new documents. Opened files keep the format detected from their content.",
    );
}

/// Settings > Grid: row appearance.
pub(super) fn grid(ui: &mut Ui, settings: &mut Settings, language: Language, changed: &mut bool) {
    form_grid(ui, "settings-grid", |ui| {
        ui.label("");
        if toggle(
            ui,
            tr(language, "Alternating Row Colors"),
            &mut settings.grid_alternating_rows,
        )
        .changed()
        {
            *changed = true;
        }
        ui.end_row();

        ui.label(tr(language, "Row Height"));
        let labels = [
            tr(language, "Compact"),
            tr(language, "Default"),
            tr(language, "Comfortable"),
        ];
        let current = settings.row_height();
        let selected = ROW_HEIGHTS
            .iter()
            .position(|height| *height == current)
            .unwrap_or(1);
        if let Some(index) = Picker::new(
            (ui.ctx().viewport_id(), "settings-row-height"),
            tr(language, "Row Height"),
            labels[selected],
            &labels,
        )
        .width(210.0)
        .show(ui)
        {
            settings.grid_row_height = ROW_HEIGHTS[index];
            *changed = true;
        }
        ui.end_row();

        ui.label("");
        if toggle(
            ui,
            tr(language, "Show Row Numbers"),
            &mut settings.grid_shows_row_numbers,
        )
        .changed()
        {
            *changed = true;
        }
        ui.end_row();
    });
}

/// Settings > Large Files: the size that opens in large mode.
pub(super) fn large_files(
    ui: &mut Ui,
    settings: &mut Settings,
    language: Language,
    changed: &mut bool,
) {
    form_grid(ui, "settings-large-files", |ui| {
        ui.label(tr(language, "Large File Threshold"));
        let labels: Vec<String> = LARGE_FILE_THRESHOLDS_MIB
            .iter()
            .map(|mebibytes| threshold_label(language, *mebibytes))
            .collect();
        let labels: Vec<&str> = labels.iter().map(String::as_str).collect();
        let selected = LARGE_FILE_THRESHOLDS_MIB
            .iter()
            .position(|mebibytes| *mebibytes == settings.large_file_threshold_mib)
            .unwrap_or_else(default_threshold_index);
        if let Some(index) = Picker::new(
            (ui.ctx().viewport_id(), "settings-large-threshold"),
            tr(language, "Large File Threshold"),
            labels[selected],
            &labels,
        )
        .width(210.0)
        .show(ui)
        {
            settings.large_file_threshold_mib = LARGE_FILE_THRESHOLDS_MIB[index];
            *changed = true;
        }
        ui.end_row();
    });
    caption(
        ui,
        language,
        "Files at or above this size open in large file mode.",
    );
}

fn form_grid(ui: &mut Ui, id: &'static str, add_contents: impl FnOnce(&mut Ui)) {
    Grid::new((ui.ctx().viewport_id(), id))
        .num_columns(2)
        .spacing([12.0, 8.0])
        .min_col_width(150.0)
        .show(ui, add_contents);
}

fn caption(ui: &mut Ui, language: Language, key: &str) {
    let theme = current(ui.ctx());
    ui.add_space(6.0);
    ui.label(
        RichText::new(tr(language, key))
            .size(theme.typography.caption)
            .color(theme.colors.secondary_label),
    );
}
