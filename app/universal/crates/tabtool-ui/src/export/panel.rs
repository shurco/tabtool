//! The export options sheet.

use core::hash::Hash;

use egui::{Align, Grid, Layout, RichText, Ui};
use tabtool_core::{CsvConfiguration, Encoding};
use tabtool_i18n::{Language, tr};

use crate::format_options::{DECIMALS, QuoteStyle, SEPARATORS};
use crate::theme::current;
use crate::widgets::{ButtonKind, Picker, PushButton, sheet, toggle};

use super::ExportOptions;

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
    options: &mut ExportOptions,
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
                    encoding_picker(ui, options, language);
                    ui.end_row();

                    ui.label(tr(language, "Separator"));
                    separator_picker(ui, options, language);
                    ui.end_row();

                    ui.label(tr(language, "Quote"));
                    quote_picker(ui, options, language);
                    ui.end_row();

                    ui.label(tr(language, "Decimal"));
                    decimal_picker(ui, options, language);
                    ui.end_row();

                    ui.label(tr(language, "Header"));
                    let mut header = options.configuration.first_row_as_header;
                    let response = toggle(ui, "", &mut header)
                        .accessible_name(tr(language, "Write Header Row"))
                        .on_hover_text(tr(language, "Write Header Row"));
                    if response.changed() {
                        options.configuration.first_row_as_header = header;
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
        Some(ExportPanelAction::Export(options.configuration.clone()))
    } else {
        None
    }
}

fn encoding_picker(ui: &mut Ui, options: &mut ExportOptions, language: Language) {
    let values: Vec<&str> = Encoding::ALL
        .iter()
        .map(|encoding| tr(language, encoding.display_name()))
        .collect();
    let selected = tr(language, options.configuration.encoding.display_name());
    if let Some(index) = Picker::new(
        (ui.ctx().viewport_id(), "export-encoding"),
        tr(language, "Encoding"),
        selected,
        &values,
    )
    .width(190.0)
    .show(ui)
    {
        options.configuration.encoding = Encoding::ALL[index];
    }
}

fn separator_picker(ui: &mut Ui, options: &mut ExportOptions, language: Language) {
    let values: Vec<&str> = SEPARATORS
        .iter()
        .map(|(_, key)| tr(language, key))
        .collect();
    let selected = SEPARATORS
        .iter()
        .find(|(separator, _)| *separator == options.configuration.column_separator)
        .map_or(
            options.configuration.column_separator.as_str(),
            |(_, key)| tr(language, key),
        );
    if let Some(index) = Picker::new(
        (ui.ctx().viewport_id(), "export-separator"),
        tr(language, "Separator"),
        selected,
        &values,
    )
    .width(190.0)
    .show(ui)
    {
        options.configuration.column_separator = SEPARATORS[index].0.to_owned();
    }
}

fn quote_picker(ui: &mut Ui, options: &mut ExportOptions, language: Language) {
    let values = [
        tr(language, "Doubled"),
        tr(language, "Backslash"),
        tr(language, "None"),
    ];
    let style = QuoteStyle::of(&options.configuration);
    if let Some(index) = Picker::new(
        (ui.ctx().viewport_id(), "export-quote"),
        tr(language, "Quote"),
        values[style.index()],
        &values,
    )
    .width(190.0)
    .show(ui)
    {
        QuoteStyle::at(index).apply(&mut options.configuration);
    }
}

fn decimal_picker(ui: &mut Ui, options: &mut ExportOptions, language: Language) {
    let values: Vec<&str> = DECIMALS.iter().map(|(_, key)| tr(language, key)).collect();
    let selected = DECIMALS
        .iter()
        .find(|(mark, _)| *mark == options.configuration.decimal_mark)
        .map_or(options.configuration.decimal_mark.as_str(), |(_, key)| {
            tr(language, key)
        });
    if let Some(index) = Picker::new(
        (ui.ctx().viewport_id(), "export-decimal"),
        tr(language, "Decimal"),
        selected,
        &values,
    )
    .width(190.0)
    .show(ui)
    {
        options.configuration.decimal_mark = DECIMALS[index].0.to_owned();
    }
}
