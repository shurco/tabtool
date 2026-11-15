//! The Settings sheet: one scrollable form with a section per group.

use egui::Ui;
use tabtool_core::Encoding;
use tabtool_i18n::{Language, tr};
use tabtool_settings::{LARGE_FILE_THRESHOLDS_MIB, QuoteStyle, Settings};

use super::{
    SheetStyle, caption, form_row, section_title, touch_picker, touch_toggle, touch_wide_button,
};
use crate::format_options::{DECIMALS, SEPARATORS};
use crate::mobile::{MOBILE_ROW_HEIGHTS, SheetOptions, sheet};
use crate::settings::options::{
    decimal_index, default_encoding_index, default_threshold_index, separator_index,
    threshold_label,
};
use crate::widgets::ButtonKind;

/// What the Settings sheet asked its host to do.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SettingsSheetResponse {
    /// The close button asked to dismiss the sheet.
    pub close_requested: bool,
    /// At least one preference changed; the app persists and applies it.
    pub changed: bool,
    /// The About row was pressed.
    pub about: bool,
}

/// Draws the Settings sheet and edits `settings` in place.
///
/// The language control takes effect on the next frame: the app resolves the
/// effective language from the settings it passes in, so no restart is needed.
pub fn settings_sheet(
    ui: &mut Ui,
    style: SheetStyle,
    settings: &mut Settings,
) -> SettingsSheetResponse {
    let language = style.language;
    let mut changed = false;
    let mut about = false;

    let response = sheet(
        ui.ctx(),
        "mobile-settings-sheet",
        style.metrics,
        SheetOptions::new(tr(language, "Settings"), tr(language, "Done")),
        |ui| {
            section_title(ui, style, tr(language, "General"));
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
            if let Some(index) = form_row(ui, style, tr(language, "Language"), |ui| {
                touch_picker(
                    ui,
                    style,
                    ("mobile-settings-language", ui.ctx().viewport_id()),
                    tr(language, "Language"),
                    names[selected],
                    &names,
                    true,
                )
            }) {
                settings.app_language = Language::ALL[index];
                changed = true;
            }

            section_title(ui, style, tr(language, "New Document"));
            let names: Vec<&str> = Encoding::ALL
                .iter()
                .map(|encoding| tr(language, encoding.display_name()))
                .collect();
            let selected = Encoding::ALL
                .iter()
                .position(|encoding| *encoding == settings.new_document_encoding)
                .unwrap_or_else(default_encoding_index);
            if let Some(index) = form_row(ui, style, tr(language, "Encoding"), |ui| {
                touch_picker(
                    ui,
                    style,
                    ("mobile-settings-encoding", ui.ctx().viewport_id()),
                    tr(language, "Encoding"),
                    names[selected],
                    &names,
                    true,
                )
            }) {
                settings.new_document_encoding = Encoding::ALL[index];
                changed = true;
            }

            let labels: Vec<&str> = SEPARATORS
                .iter()
                .map(|(_, key)| tr(language, key))
                .collect();
            let selected = separator_index(&settings.new_document_separator);
            if let Some(index) = form_row(ui, style, tr(language, "Separator"), |ui| {
                touch_picker(
                    ui,
                    style,
                    ("mobile-settings-separator", ui.ctx().viewport_id()),
                    tr(language, "Separator"),
                    labels[selected],
                    &labels,
                    true,
                )
            }) {
                settings.new_document_separator = SEPARATORS[index].0.to_owned();
                changed = true;
            }

            let quotes = [
                tr(language, "Doubled"),
                tr(language, "Backslash"),
                tr(language, "None"),
            ];
            let selected = QuoteStyle::ALL
                .iter()
                .position(|style| *style == settings.new_document_quote_style)
                .unwrap_or(0);
            if let Some(index) = form_row(ui, style, tr(language, "Quote"), |ui| {
                touch_picker(
                    ui,
                    style,
                    ("mobile-settings-quote", ui.ctx().viewport_id()),
                    tr(language, "Quote"),
                    quotes[selected],
                    &quotes,
                    true,
                )
            }) {
                settings.new_document_quote_style = QuoteStyle::ALL[index];
                changed = true;
            }

            let decimals: Vec<&str> = DECIMALS.iter().map(|(_, key)| tr(language, key)).collect();
            let selected = decimal_index(&settings.new_document_decimal_mark);
            if let Some(index) = form_row(ui, style, tr(language, "Decimal Mark"), |ui| {
                touch_picker(
                    ui,
                    style,
                    ("mobile-settings-decimal", ui.ctx().viewport_id()),
                    tr(language, "Decimal Mark"),
                    decimals[selected],
                    &decimals,
                    true,
                )
            }) {
                settings.new_document_decimal_mark = DECIMALS[index].0.to_owned();
                changed = true;
            }

            if touch_toggle(
                ui,
                style,
                tr(language, "First Row as Header"),
                &mut settings.new_document_first_row_as_header,
                true,
            )
            .changed()
            {
                changed = true;
            }
            caption(
                ui,
                style,
                tr(
                    language,
                    "These apply to new documents. Opened files keep the format detected from their content.",
                ),
            );

            section_title(ui, style, tr(language, "Grid"));
            if touch_toggle(
                ui,
                style,
                tr(language, "Alternating Row Colors"),
                &mut settings.grid_alternating_rows,
                true,
            )
            .changed()
            {
                changed = true;
            }

            let heights = [
                tr(language, "Compact"),
                tr(language, "Default"),
                tr(language, "Comfortable"),
            ];
            let current = tabtool_settings::valid_mobile_row_height(settings.grid_row_height);
            let selected = MOBILE_ROW_HEIGHTS
                .iter()
                .position(|height| *height == current)
                .unwrap_or(1);
            if let Some(index) = form_row(ui, style, tr(language, "Row Height"), |ui| {
                touch_picker(
                    ui,
                    style,
                    ("mobile-settings-row-height", ui.ctx().viewport_id()),
                    tr(language, "Row Height"),
                    heights[selected],
                    &heights,
                    true,
                )
            }) {
                settings.grid_row_height = MOBILE_ROW_HEIGHTS[index];
                changed = true;
            }

            if touch_toggle(
                ui,
                style,
                tr(language, "Show Row Numbers"),
                &mut settings.grid_shows_row_numbers,
                true,
            )
            .changed()
            {
                changed = true;
            }

            section_title(ui, style, tr(language, "Large Files"));
            let labels: Vec<String> = LARGE_FILE_THRESHOLDS_MIB
                .iter()
                .map(|mebibytes| threshold_label(language, *mebibytes))
                .collect();
            let labels: Vec<&str> = labels.iter().map(String::as_str).collect();
            let selected = LARGE_FILE_THRESHOLDS_MIB
                .iter()
                .position(|mebibytes| *mebibytes == settings.large_file_threshold_mib)
                .unwrap_or_else(default_threshold_index);
            if let Some(index) = form_row(ui, style, tr(language, "Large File Threshold"), |ui| {
                touch_picker(
                    ui,
                    style,
                    ("mobile-settings-large-threshold", ui.ctx().viewport_id()),
                    tr(language, "Large File Threshold"),
                    labels[selected],
                    &labels,
                    true,
                )
            }) {
                settings.large_file_threshold_mib = LARGE_FILE_THRESHOLDS_MIB[index];
                changed = true;
            }
            caption(
                ui,
                style,
                tr(
                    language,
                    "Files at or above this size open in large file mode.",
                ),
            );

            section_title(ui, style, tr(language, "About"));
            if touch_wide_button(ui, style, tr(language, "About"), ButtonKind::Bordered).clicked() {
                about = true;
            }
            ui.add_space(8.0);
        },
    );

    SettingsSheetResponse {
        close_requested: response.close_requested,
        changed,
        about,
    }
}
