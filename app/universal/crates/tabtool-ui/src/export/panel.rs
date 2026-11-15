//! The export options sheet.

use core::hash::Hash;

use egui::{Align, Grid, Layout, RichText, Ui};
use tabtool_core::{CsvConfiguration, ExportFormat};
use tabtool_i18n::{Language, tr};

use crate::theme::current;
use crate::widgets::{ButtonKind, PushButton, sheet, toggle};

use super::controls::{
    PickerSize, decimal_picker, encoding_picker, format_picker, quote_picker, separator_picker,
};

/// What the export panel asked for.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExportPanelAction {
    /// Close the panel without exporting.
    Cancel,
    /// Write the file with this candidate format and configuration.
    Export(ExportFormat, CsvConfiguration),
}

/// Draws the export options sheet.
///
/// JSON and XML use only the decimal control; the CSV-only rows stay visible
/// but disabled so the sheet keeps its size.
pub fn export_panel(
    ui: &mut Ui,
    id_salt: impl Hash + std::fmt::Debug,
    format: &mut ExportFormat,
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
                    ui.label(tr(language, "Format"));
                    format_picker(ui, format, language, PickerSize::DESKTOP);
                    ui.end_row();

                    let csv = *format == ExportFormat::Csv;
                    csv_row(ui, csv, tr(language, "Encoding"), |ui| {
                        encoding_picker(ui, configuration, language, PickerSize::DESKTOP);
                    });
                    csv_row(ui, csv, tr(language, "Separator"), |ui| {
                        separator_picker(ui, configuration, language, PickerSize::DESKTOP);
                    });
                    csv_row(ui, csv, tr(language, "Quote"), |ui| {
                        quote_picker(ui, configuration, language, PickerSize::DESKTOP);
                    });

                    ui.label(tr(language, "Decimal"));
                    decimal_picker(ui, configuration, language, PickerSize::DESKTOP);
                    ui.end_row();

                    csv_row(ui, csv, tr(language, "Header"), |ui| {
                        let mut header = configuration.first_row_as_header;
                        let response = toggle(ui, "", &mut header)
                            .accessible_name(tr(language, "Write Header Row"))
                            .on_hover_text(tr(language, "Write Header Row"));
                        if response.changed() {
                            configuration.first_row_as_header = header;
                        }
                    });
                });

            ui.add_space(4.0);
            ui.label(
                RichText::new(tr(language, caption_key(*format)))
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
        Some(ExportPanelAction::Export(*format, configuration.clone()))
    } else {
        None
    }
}

/// The catalog key of the caption under the options.
pub(crate) fn caption_key(format: ExportFormat) -> &'static str {
    match format {
        ExportFormat::Csv => "The header is written only to the exported file.",
        ExportFormat::Json | ExportFormat::Xml => {
            "Column names become keys. The file is written in UTF-8."
        }
    }
}

/// A grid row that only applies to CSV; disabled, not hidden, for JSON and XML.
fn csv_row(ui: &mut Ui, csv: bool, label: &str, control: impl FnOnce(&mut Ui)) {
    ui.add_enabled_ui(csv, |ui| ui.label(label));
    ui.add_enabled_ui(csv, control);
    ui.end_row();
}
