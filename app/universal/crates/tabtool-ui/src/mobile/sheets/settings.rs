//! The Settings sheet: one scrollable form with a section per group.

use egui::Ui;
use tabtool_i18n::tr;
use tabtool_settings::Settings;

use super::{
    SheetStyle, caption, form_row, section_title, touch_picker, touch_toggle, touch_wide_button,
};
use crate::mobile::{MOBILE_ROW_HEIGHTS, SheetOptions, sheet};
use crate::settings::options::SettingPicker;
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
            changed |= picker_row(
                ui,
                style,
                "mobile-settings-language",
                SettingPicker::Language,
                settings,
            );

            section_title(ui, style, tr(language, "New Document"));
            for (id, picker) in [
                ("mobile-settings-encoding", SettingPicker::Encoding),
                ("mobile-settings-separator", SettingPicker::Separator),
                ("mobile-settings-quote", SettingPicker::Quote),
                ("mobile-settings-decimal", SettingPicker::DecimalMark),
            ] {
                changed |= picker_row(ui, style, id, picker, settings);
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

            changed |= picker_row(
                ui,
                style,
                "mobile-settings-row-height",
                SettingPicker::RowHeight(&MOBILE_ROW_HEIGHTS),
                settings,
            );

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
            changed |= picker_row(
                ui,
                style,
                "mobile-settings-large-threshold",
                SettingPicker::LargeFileThreshold,
                settings,
            );
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

/// One labelled picker row; reports whether the user chose an option.
fn picker_row(
    ui: &mut Ui,
    style: SheetStyle,
    id: &'static str,
    picker: SettingPicker,
    settings: &mut Settings,
) -> bool {
    let title = tr(style.language, picker.title());
    let labels = picker.labels(style.language);
    let labels: Vec<&str> = labels.iter().map(String::as_str).collect();
    let selected = picker.selected(settings);
    let chosen = form_row(ui, style, title, |ui| {
        touch_picker(
            ui,
            style,
            (id, ui.ctx().viewport_id()),
            title,
            labels[selected],
            &labels,
            true,
        )
    });
    if let Some(index) = chosen {
        picker.apply(settings, index);
    }
    chosen.is_some()
}
