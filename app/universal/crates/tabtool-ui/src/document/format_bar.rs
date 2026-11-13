//! The format controls at the bottom of the document window.
//!
//! The bar reports a complete candidate configuration for every edit; the view
//! derives the operation with `plan_format_change`, so this module never
//! changes the document directly.

use egui::{Frame, Margin, RichText, Stroke, Ui};
use tabtool_core::{CsvConfiguration, Encoding};
use tabtool_document::Document;
use tabtool_i18n::{Language, tr};

use crate::format_options::{DECIMALS, QuoteStyle, SEPARATORS};
use crate::theme::current;
use crate::widgets::{ButtonKind, Picker, PushButton, toggle_enabled};

/// What one format-bar interaction asks for.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FormatBarAction {
    /// Apply a complete candidate configuration.
    Configuration(CsvConfiguration),
    /// Detect the format from the file on disk.
    Detect,
}

/// Draws the format bar; `None` when nothing was changed.
pub fn show(
    ui: &mut Ui,
    document: &Document,
    language: Language,
    busy: bool,
    detect_available: bool,
) -> Option<FormatBarAction> {
    let theme = current(ui.ctx());
    let configuration = document.configuration();
    let large = document.is_large_mode();
    let mut chosen = None;

    let frame = Frame::new()
        .fill(theme.colors.window_background)
        .inner_margin(Margin::symmetric(theme.metrics.toolbar_padding_x as i8, 6))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing.x = 10.0;
            ui.horizontal(|ui| {
                control(ui, tr(language, "Encoding"), |ui| {
                    let values: Vec<&str> = Encoding::ALL
                        .iter()
                        .map(|encoding| tr(language, encoding.display_name()))
                        .collect();
                    let selected = tr(language, configuration.encoding.display_name());
                    if let Some(index) = Picker::new(
                        (ui.ctx().viewport_id(), "format-encoding"),
                        tr(language, "Encoding"),
                        selected,
                        &values,
                    )
                    .width(165.0)
                    .enabled(!busy)
                    .show(ui)
                    {
                        let mut candidate = configuration.clone();
                        candidate.encoding = Encoding::ALL[index];
                        chosen = Some(FormatBarAction::Configuration(candidate));
                    }
                });

                control(ui, tr(language, "Separator"), |ui| {
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
                    if let Some(index) = Picker::new(
                        (ui.ctx().viewport_id(), "format-separator"),
                        tr(language, "Separator"),
                        selected,
                        &values,
                    )
                    .width(94.0)
                    .enabled(!busy)
                    .show(ui)
                    {
                        let mut candidate = configuration.clone();
                        candidate.column_separator = SEPARATORS[index].0.to_owned();
                        chosen = Some(FormatBarAction::Configuration(candidate));
                    }
                });

                control(ui, tr(language, "Quote"), |ui| {
                    let values = [
                        tr(language, "Doubled"),
                        tr(language, "Backslash"),
                        tr(language, "None"),
                    ];
                    let style = QuoteStyle::of(configuration);
                    if let Some(index) = Picker::new(
                        (ui.ctx().viewport_id(), "format-quote"),
                        tr(language, "Quote"),
                        values[style.index()],
                        &values,
                    )
                    .width(102.0)
                    .enabled(!busy)
                    .show(ui)
                    {
                        let mut candidate = configuration.clone();
                        QuoteStyle::at(index).apply(&mut candidate);
                        chosen = Some(FormatBarAction::Configuration(candidate));
                    }
                });

                control(ui, tr(language, "Decimal"), |ui| {
                    let values: Vec<&str> =
                        DECIMALS.iter().map(|(_, key)| tr(language, key)).collect();
                    let selected = DECIMALS
                        .iter()
                        .find(|(mark, _)| *mark == configuration.decimal_mark)
                        .map_or(configuration.decimal_mark.as_str(), |(_, key)| {
                            tr(language, key)
                        });
                    let response = Picker::new(
                        (ui.ctx().viewport_id(), "format-decimal"),
                        tr(language, "Decimal"),
                        selected,
                        &values,
                    )
                    .width(84.0)
                    .enabled(!busy && !large)
                    .show(ui);
                    if let Some(index) = response {
                        let mut candidate = configuration.clone();
                        candidate.decimal_mark = DECIMALS[index].0.to_owned();
                        chosen = Some(FormatBarAction::Configuration(candidate));
                    }
                });

                control(ui, tr(language, "Header"), |ui| {
                    let mut header = configuration.first_row_as_header;
                    let response = toggle_enabled(ui, "", &mut header, !busy);
                    let response = response
                        .accessible_name(tr(language, "Header"))
                        .on_hover_text(tr(language, "Header"));
                    if response.changed() {
                        let mut candidate = configuration.clone();
                        candidate.first_row_as_header = header;
                        chosen = Some(FormatBarAction::Configuration(candidate));
                    }
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let detect = PushButton::new("\u{2728}", ButtonKind::Plain)
                        .enabled(!busy && detect_available)
                        .accessible_name(tr(language, "Detect Format Automatically"))
                        .show(ui)
                        .on_hover_text(tr(language, "Detect Format Automatically"));
                    if detect.clicked() {
                        chosen = Some(FormatBarAction::Detect);
                    }
                });
            });
        });

    let rect = frame.response.rect;
    ui.painter().hline(
        rect.x_range(),
        rect.top(),
        Stroke::new(1.0, theme.colors.separator),
    );
    chosen
}

/// A compact caption above a control, keeping the bar usable at 640 points.
fn control<R>(ui: &mut Ui, caption: &str, add: impl FnOnce(&mut Ui) -> R) -> R {
    let theme = current(ui.ctx());
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 2.0;
        ui.label(
            RichText::new(caption)
                .size(theme.typography.caption)
                .color(theme.colors.secondary_label),
        );
        add(ui)
    })
    .inner
}
