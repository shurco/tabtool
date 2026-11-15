//! The four Settings tab forms.

use egui::{Grid, RichText, Ui};
use tabtool_i18n::{Language, tr};
use tabtool_settings::{ROW_HEIGHTS, Settings};

use crate::theme::current;
use crate::widgets::{Picker, toggle};

use super::options::SettingPicker;

/// Settings > General: language and the format bar.
pub(super) fn general(
    ui: &mut Ui,
    settings: &mut Settings,
    language: Language,
    changed: &mut bool,
) {
    form_grid(ui, "settings-general", |ui| {
        picker_row(
            ui,
            "settings-language",
            SettingPicker::Language,
            settings,
            language,
            changed,
        );

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
        for (id, picker) in [
            ("settings-encoding", SettingPicker::Encoding),
            ("settings-separator", SettingPicker::Separator),
            ("settings-quote", SettingPicker::Quote),
            ("settings-decimal", SettingPicker::DecimalMark),
        ] {
            picker_row(ui, id, picker, settings, language, changed);
        }

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

        picker_row(
            ui,
            "settings-row-height",
            SettingPicker::RowHeight(&ROW_HEIGHTS),
            settings,
            language,
            changed,
        );

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
        picker_row(
            ui,
            "settings-large-threshold",
            SettingPicker::LargeFileThreshold,
            settings,
            language,
            changed,
        );
    });
    caption(
        ui,
        language,
        "Files at or above this size open in large file mode.",
    );
}

/// One labelled picker row of a form grid.
fn picker_row(
    ui: &mut Ui,
    id: &'static str,
    picker: SettingPicker,
    settings: &mut Settings,
    language: Language,
    changed: &mut bool,
) {
    let title = tr(language, picker.title());
    ui.label(title);
    let labels = picker.labels(language);
    let labels: Vec<&str> = labels.iter().map(String::as_str).collect();
    if let Some(index) = Picker::new(
        (ui.ctx().viewport_id(), id),
        title,
        labels[picker.selected(settings)],
        &labels,
    )
    .width(210.0)
    .show(ui)
    {
        picker.apply(settings, index);
        *changed = true;
    }
    ui.end_row();
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
