//! The export options sheet.

use core::hash::Hash;

use egui::{Align, Grid, Layout, RichText, Ui};
use tabtool_core::CsvConfiguration;
use tabtool_i18n::{Language, tr};

use crate::theme::current;
use crate::widgets::{ButtonKind, PushButton, sheet, toggle};

use super::controls::{
    PickerSize, decimal_picker, encoding_picker, quote_picker, separator_picker,
};

/// What the export panel asked for.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExportPanelAction {
    /// Close the panel without exporting.
    Cancel,
    /// Write the file with this candidate configuration.
    Export(CsvConfiguration),
}

/// Draws the export options sheet.
pub fn export_panel(
    ui: &mut Ui,
    id_salt: impl Hash + std::fmt::Debug,
    configuration: &mut CsvConfiguration,
    language: Language,
) -> Option<ExportPanelAction> {
    let theme = current(ui.ctx());
    let response = sheet(
        ui,
        id_salt,
        tr(language, "Export"),
        tr(language, "Cancel"),
        |ui| {
            let mut pressed = false;
            Grid::new((ui.ctx().viewport_id(), "export-options-grid"))
                .num_columns(2)
                .spacing([12.0, 8.0])
                .show(ui, |ui| {
                    ui.label(tr(language, "Encoding"));
                    encoding_picker(ui, configuration, language, PickerSize::DESKTOP);
                    ui.end_row();

                    ui.label(tr(language, "Separator"));
                    separator_picker(ui, configuration, language, PickerSize::DESKTOP);
                    ui.end_row();

                    ui.label(tr(language, "Quote"));
                    quote_picker(ui, configuration, language, PickerSize::DESKTOP);
                    ui.end_row();

                    ui.label(tr(language, "Decimal"));
                    decimal_picker(ui, configuration, language, PickerSize::DESKTOP);
                    ui.end_row();

                    ui.label(tr(language, "Header"));
                    let mut header = configuration.first_row_as_header;
                    let response = toggle(ui, "", &mut header)
                        .accessible_name(tr(language, "Write Header Row"))
                        .on_hover_text(tr(language, "Write Header Row"));
                    if response.changed() {
                        configuration.first_row_as_header = header;
                    }
                    ui.end_row();
                });

            ui.add_space(4.0);
            ui.label(
                RichText::new(tr(
                    language,
                    "The header is written only to the exported file.",
                ))
                .size(theme.typography.caption)
                .color(theme.colors.secondary_label),
            );
            ui.add_space(8.0);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if PushButton::new(tr(language, "Export"), ButtonKind::Prominent)
                    .show(ui)
                    .clicked()
                {
                    pressed = true;
                }
            });
            pressed
        },
    );

    if response.close_requested {
        Some(ExportPanelAction::Cancel)
    } else if response.inner {
        Some(ExportPanelAction::Export(configuration.clone()))
    } else {
        None
    }
}
