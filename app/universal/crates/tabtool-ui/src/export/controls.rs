//! The export option pickers, shared by the desktop panel and the touch sheet.

use egui::Ui;
use tabtool_core::{CsvConfiguration, Encoding, ExportFormat};
use tabtool_i18n::{Language, tr};
use tabtool_settings::QuoteStyle;

use crate::format_options::{DECIMALS, SEPARATORS};
use crate::widgets::Picker;

/// The closed picker geometry.
#[derive(Clone, Copy, Debug)]
pub(crate) struct PickerSize {
    /// The button width in points.
    pub width: f32,
    /// The touch height, or the desktop default when `None`.
    pub height: Option<f32>,
    /// The label size, or the theme body size when `None`.
    pub text_size: Option<f32>,
}

impl PickerSize {
    /// The original desktop geometry.
    pub(crate) const DESKTOP: Self = Self {
        width: 190.0,
        height: None,
        text_size: None,
    };
}

pub(crate) fn format_picker(
    ui: &mut Ui,
    format: &mut ExportFormat,
    language: Language,
    size: PickerSize,
) {
    let values = ExportFormat::ALL.map(ExportFormat::name);
    if let Some(index) = picker(
        ui,
        "export-format",
        tr(language, "Format"),
        format.name(),
        &values,
        size,
    ) {
        *format = ExportFormat::ALL[index];
    }
}

pub(crate) fn encoding_picker(
    ui: &mut Ui,
    configuration: &mut CsvConfiguration,
    language: Language,
    size: PickerSize,
) {
    let values: Vec<&str> = Encoding::ALL
        .iter()
        .map(|encoding| tr(language, encoding.display_name()))
        .collect();
    let selected = tr(language, configuration.encoding.display_name());
    if let Some(index) = picker(
        ui,
        "export-encoding",
        tr(language, "Encoding"),
        selected,
        &values,
        size,
    ) {
        configuration.encoding = Encoding::ALL[index];
    }
}

pub(crate) fn separator_picker(
    ui: &mut Ui,
    configuration: &mut CsvConfiguration,
    language: Language,
    size: PickerSize,
) {
    let values: Vec<&str> = SEPARATORS
        .iter()
        .map(|(_, key)| tr(language, key))
        .collect();
    let selected = SEPARATORS
        .iter()
        .find(|(separator, _)| *separator == configuration.column_separator)
        .map_or(configuration.column_separator.as_str(), |(_, key)| {
            tr(language, key)
        });
    if let Some(index) = picker(
        ui,
        "export-separator",
        tr(language, "Separator"),
        selected,
        &values,
        size,
    ) {
        configuration.column_separator = SEPARATORS[index].0.to_owned();
    }
}

pub(crate) fn quote_picker(
    ui: &mut Ui,
    configuration: &mut CsvConfiguration,
    language: Language,
    size: PickerSize,
) {
    let values = [
        tr(language, "Doubled"),
        tr(language, "Backslash"),
        tr(language, "None"),
    ];
    let style = QuoteStyle::of(configuration);
    if let Some(index) = picker(
        ui,
        "export-quote",
        tr(language, "Quote"),
        values[style.index()],
        &values,
        size,
    ) {
        QuoteStyle::ALL[index].apply(configuration);
    }
}

pub(crate) fn decimal_picker(
    ui: &mut Ui,
    configuration: &mut CsvConfiguration,
    language: Language,
    size: PickerSize,
) {
    let values: Vec<&str> = DECIMALS.iter().map(|(_, key)| tr(language, key)).collect();
    let selected = DECIMALS
        .iter()
        .find(|(mark, _)| *mark == configuration.decimal_mark)
        .map_or(configuration.decimal_mark.as_str(), |(_, key)| {
            tr(language, key)
        });
    if let Some(index) = picker(
        ui,
        "export-decimal",
        tr(language, "Decimal"),
        selected,
        &values,
        size,
    ) {
        configuration.decimal_mark = DECIMALS[index].0.to_owned();
    }
}

fn picker(
    ui: &mut Ui,
    id: &'static str,
    name: &str,
    selected: &str,
    values: &[&str],
    size: PickerSize,
) -> Option<usize> {
    let mut picker =
        Picker::new((ui.ctx().viewport_id(), id), name, selected, values).width(size.width);
    if let Some(height) = size.height {
        picker = picker.height(height);
    }
    if let Some(text_size) = size.text_size {
        picker = picker.text_size(text_size);
    }
    picker.show(ui)
}
