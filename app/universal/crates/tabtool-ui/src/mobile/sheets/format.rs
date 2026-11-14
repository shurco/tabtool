//! The Format sheet: parsing controls, header mode and automatic detection.

use egui::Ui;
use tabtool_core::{CsvConfiguration, Encoding};
use tabtool_document::{Document, Storage};
use tabtool_i18n::{Language, tr};

use super::{
    SheetScreenResponse, SheetStyle, caption, form_row, touch_button, touch_picker, touch_toggle,
};
use crate::document::{
    ChromeAlert, FormatApplyOutcome, apply_format_configuration, detect_format_configuration,
    is_document_busy, request_format_configuration,
};
use crate::format_options::{DECIMALS, QuoteStyle, SEPARATORS};
use crate::mobile::{SheetOptions, sheet};
use crate::widgets::{AlertAction, BannerKind, ButtonKind, banner, modal};

/// What one format interaction asks for.
enum FormatChoice {
    /// Apply a complete candidate configuration.
    Configuration(CsvConfiguration),
    /// Detect the format from the file on disk.
    Detect,
}

/// The Format sheet's confirmation and failure state.
#[derive(Default)]
pub struct FormatSheet {
    pending: Option<CsvConfiguration>,
    alert: Option<ChromeAlert>,
}

impl FormatSheet {
    /// A sheet with no pending confirmation.
    pub fn new() -> Self {
        Self::default()
    }

    /// Draws the sheet; the document changes as controls are chosen.
    pub fn show(
        &mut self,
        ui: &mut Ui,
        style: SheetStyle,
        document: &mut Document,
        storage: &dyn Storage,
        prefers_chinese: bool,
    ) -> SheetScreenResponse {
        let language = style.language;
        let busy = is_document_busy(document);
        let large = document.is_large_mode();
        let configuration = document.configuration().clone();
        let detect_available = document.file_path().is_some();
        let alert = self.alert.clone();
        let mut dismiss_alert = false;
        let mut chosen = None;

        let response = sheet(
            ui.ctx(),
            "mobile-format-sheet",
            style.metrics,
            SheetOptions::new(tr(language, "Format"), tr(language, "Done"))
                .keyboard_height(style.keyboard_height),
            |ui| {
                if let Some(alert) = &alert {
                    let shown = banner(
                        ui,
                        BannerKind::Error,
                        tr(language, alert.title),
                        &alert.message,
                        Some(tr(language, "OK")),
                    );
                    if shown.action_clicked {
                        dismiss_alert = true;
                    }
                    ui.add_space(8.0);
                }

                let names: Vec<&str> = Encoding::ALL
                    .iter()
                    .map(|encoding| tr(language, encoding.display_name()))
                    .collect();
                let selected = tr(language, configuration.encoding.display_name());
                if let Some(index) = form_row(ui, style, tr(language, "Encoding"), |ui| {
                    touch_picker(
                        ui,
                        style,
                        ("mobile-format-encoding", ui.ctx().viewport_id()),
                        tr(language, "Encoding"),
                        selected,
                        &names,
                        !busy,
                    )
                }) {
                    let mut candidate = configuration.clone();
                    candidate.encoding = Encoding::ALL[index];
                    chosen = Some(FormatChoice::Configuration(candidate));
                }

                let labels: Vec<&str> = SEPARATORS
                    .iter()
                    .map(|(_, key)| tr(language, key))
                    .collect();
                let selected = SEPARATORS
                    .iter()
                    .find(|(separator, _)| *separator == configuration.column_separator)
                    .map_or(configuration.column_separator.as_str(), |(_, key)| {
                        tr(language, key)
                    });
                if let Some(index) = form_row(ui, style, tr(language, "Separator"), |ui| {
                    touch_picker(
                        ui,
                        style,
                        ("mobile-format-separator", ui.ctx().viewport_id()),
                        tr(language, "Separator"),
                        selected,
                        &labels,
                        !busy,
                    )
                }) {
                    let mut candidate = configuration.clone();
                    candidate.column_separator = SEPARATORS[index].0.to_owned();
                    chosen = Some(FormatChoice::Configuration(candidate));
                }

                let quotes = [
                    tr(language, "Doubled"),
                    tr(language, "Backslash"),
                    tr(language, "None"),
                ];
                let style_index = QuoteStyle::of(&configuration).index();
                if let Some(index) = form_row(ui, style, tr(language, "Quote"), |ui| {
                    touch_picker(
                        ui,
                        style,
                        ("mobile-format-quote", ui.ctx().viewport_id()),
                        tr(language, "Quote"),
                        quotes[style_index],
                        &quotes,
                        !busy,
                    )
                }) {
                    let mut candidate = configuration.clone();
                    QuoteStyle::at(index).apply(&mut candidate);
                    chosen = Some(FormatChoice::Configuration(candidate));
                }

                let decimals: Vec<&str> =
                    DECIMALS.iter().map(|(_, key)| tr(language, key)).collect();
                let selected = DECIMALS
                    .iter()
                    .find(|(mark, _)| *mark == configuration.decimal_mark)
                    .map_or(configuration.decimal_mark.as_str(), |(_, key)| {
                        tr(language, key)
                    });
                if let Some(index) = form_row(ui, style, tr(language, "Decimal"), |ui| {
                    touch_picker(
                        ui,
                        style,
                        ("mobile-format-decimal", ui.ctx().viewport_id()),
                        tr(language, "Decimal"),
                        selected,
                        &decimals,
                        !busy && !large,
                    )
                }) {
                    let mut candidate = configuration.clone();
                    candidate.decimal_mark = DECIMALS[index].0.to_owned();
                    chosen = Some(FormatChoice::Configuration(candidate));
                }

                let mut header = configuration.first_row_as_header;
                if touch_toggle(
                    ui,
                    style,
                    tr(language, "First Row as Header"),
                    &mut header,
                    !busy,
                )
                .changed()
                {
                    let mut candidate = configuration.clone();
                    candidate.first_row_as_header = header;
                    chosen = Some(FormatChoice::Configuration(candidate));
                }

                ui.add_space(10.0);
                if touch_button(
                    style,
                    tr(language, "Detect Automatically"),
                    ButtonKind::Bordered,
                )
                .enabled(!busy && detect_available)
                .show(ui)
                .clicked()
                {
                    chosen = Some(FormatChoice::Detect);
                }

                ui.add_space(10.0);
                caption(
                    ui,
                    style,
                    tr(language, "Changing the format re-reads the file."),
                );
                caption(
                    ui,
                    style,
                    tr(
                        language,
                        "Your unsaved changes will be discarded. The table will be replaced with the last saved version from disk.",
                    ),
                );
            },
        );

        if dismiss_alert {
            self.alert = None;
        }
        if let Some(choice) = chosen {
            self.apply(choice, document, storage, language, prefers_chinese);
        }
        self.show_confirmation(ui, style, document, storage);

        let close_requested = response.close_requested && self.pending.is_none();
        if close_requested {
            self.alert = None;
        }
        SheetScreenResponse { close_requested }
    }

    fn apply(
        &mut self,
        choice: FormatChoice,
        document: &mut Document,
        storage: &dyn Storage,
        language: Language,
        prefers_chinese: bool,
    ) {
        match choice {
            FormatChoice::Configuration(configuration) => {
                self.request(document, storage, language, configuration);
            }
            FormatChoice::Detect => {
                match detect_format_configuration(document, storage, language, prefers_chinese) {
                    Ok(detected) => self.request(document, storage, language, detected),
                    Err(alert) => self.alert = Some(alert),
                }
            }
        }
    }

    fn request(
        &mut self,
        document: &mut Document,
        storage: &dyn Storage,
        language: Language,
        configuration: CsvConfiguration,
    ) {
        match request_format_configuration(document, storage, language, configuration) {
            FormatApplyOutcome::Applied => self.alert = None,
            FormatApplyOutcome::NeedsConfirmation(configuration) => {
                self.pending = Some(configuration);
            }
            FormatApplyOutcome::Failed(alert) => self.alert = Some(alert),
        }
    }

    fn show_confirmation(
        &mut self,
        ui: &mut Ui,
        style: SheetStyle,
        document: &mut Document,
        storage: &dyn Storage,
    ) {
        let Some(configuration) = self.pending.clone() else {
            return;
        };
        let language = style.language;
        let actions = [
            AlertAction::new(tr(language, "Re-read"), ButtonKind::Destructive),
            AlertAction::new(tr(language, "Cancel"), ButtonKind::Bordered),
        ];
        let pressed = modal(
            ui,
            "mobile-format-reread",
            tr(language, "Re-read the file and discard unsaved edits?"),
            tr(
                language,
                "The table will be replaced with the last saved version from disk. The file itself is not changed.",
            ),
            &actions,
        );
        let Some(index) = pressed else {
            return;
        };
        self.pending = None;
        if index == 0
            && let Some(alert) =
                apply_format_configuration(document, storage, language, &configuration, true)
        {
            self.alert = Some(alert);
        }
    }
}
