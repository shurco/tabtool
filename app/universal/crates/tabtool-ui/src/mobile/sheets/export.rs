//! The Export sheet, the large-file Save As copy and the saved-copy notice.

use std::path::PathBuf;

use egui::{Align, Layout, Ui};
use tabtool_core::{CsvConfiguration, ExportFormat};
use tabtool_document::Document;
use tabtool_i18n::tr;

use super::{SheetScreenResponse, SheetStyle, caption, form_row, touch_button, touch_toggle};
use crate::document::can_export;
use crate::export;
use crate::export::controls::{
    PickerSize, decimal_picker, encoding_picker, format_picker, quote_picker, separator_picker,
};
use crate::mobile::{DocumentFiles, SheetOptions, sheet};
use crate::widgets::{AlertAction, BannerKind, ButtonKind, banner, modal};

mod write;

/// The catalog key of an export that did not finish.
const EXPORT_NOT_COMPLETED: &str = "Export Not Completed";

/// The Export sheet state.
pub struct ExportSheet {
    format: ExportFormat,
    configuration: CsvConfiguration,
    base_name: String,
    confirm: Option<CsvConfiguration>,
    pending: Option<PendingExport>,
    notice: Option<ExportNotice>,
    finished: bool,
}

struct PendingExport {
    source: PathBuf,
    name: String,
}

struct ExportNotice {
    title: &'static str,
    message: String,
}

impl ExportSheet {
    /// A sheet seeded from `document`; the document is never changed.
    ///
    /// The host must commit any open cell edit before presenting the sheet.
    /// The touch editor defers toolbar actions until an edit commits, so the
    /// table this sheet reads is the settled one.
    pub fn new(document: &Document) -> Self {
        Self {
            format: ExportFormat::Csv,
            configuration: document.configuration().clone(),
            base_name: export::base_name(document.file_path()),
            confirm: None,
            pending: None,
            notice: None,
            finished: false,
        }
    }

    /// Whether the platform picker still owns the export.
    pub fn is_pending(&self) -> bool {
        self.pending.is_some()
    }

    /// Draws the sheet and runs the export the user asked for.
    pub fn show(
        &mut self,
        ui: &mut Ui,
        style: SheetStyle,
        document: &Document,
        files: &dyn DocumentFiles,
    ) -> SheetScreenResponse {
        let language = style.language;
        self.poll_pending(files, style.language);

        let mut dismissed = false;
        let mut pressed = false;
        let enabled = self.pending.is_none();
        let notice = self
            .notice
            .as_ref()
            .map(|notice| (notice.title, notice.message.clone()));

        let response = sheet(
            ui.ctx(),
            "mobile-export-sheet",
            style.metrics,
            SheetOptions::new(tr(language, "Export"), tr(language, "Cancel"))
                .keyboard_height(style.keyboard_height),
            |ui| {
                if let Some((title, message)) = &notice {
                    let shown = banner(
                        ui,
                        BannerKind::Error,
                        tr(language, title),
                        message,
                        Some(tr(language, "OK")),
                    );
                    if shown.action_clicked {
                        dismissed = true;
                    }
                    ui.add_space(8.0);
                }

                let size = PickerSize {
                    width: ui.available_width(),
                    height: Some(style.metrics.control_height),
                    text_size: Some(style.metrics.body),
                };
                ui.add_enabled_ui(enabled, |ui| {
                    form_row(ui, style, tr(language, "Format"), |ui| {
                        let width = ui.available_width();
                        format_picker(ui, &mut self.format, language, PickerSize { width, ..size });
                    });
                    // CSV-only rows stay visible but disabled for JSON and XML.
                    let csv = self.format == ExportFormat::Csv;
                    ui.add_enabled_ui(csv, |ui| {
                        form_row(ui, style, tr(language, "Encoding"), |ui| {
                            let width = ui.available_width();
                            encoding_picker(
                                ui,
                                &mut self.configuration,
                                language,
                                PickerSize { width, ..size },
                            );
                        });
                        form_row(ui, style, tr(language, "Separator"), |ui| {
                            let width = ui.available_width();
                            separator_picker(
                                ui,
                                &mut self.configuration,
                                language,
                                PickerSize { width, ..size },
                            );
                        });
                        form_row(ui, style, tr(language, "Quote"), |ui| {
                            let width = ui.available_width();
                            quote_picker(
                                ui,
                                &mut self.configuration,
                                language,
                                PickerSize { width, ..size },
                            );
                        });
                    });
                    form_row(ui, style, tr(language, "Decimal"), |ui| {
                        let width = ui.available_width();
                        decimal_picker(
                            ui,
                            &mut self.configuration,
                            language,
                            PickerSize { width, ..size },
                        );
                    });

                    touch_toggle(
                        ui,
                        style,
                        tr(language, "Write Header Row"),
                        &mut self.configuration.first_row_as_header,
                        csv,
                    );
                    caption(ui, style, tr(language, export::caption_key(self.format)));

                    ui.add_space(12.0);
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if touch_button(style, tr(language, "Export"), ButtonKind::Prominent)
                            .enabled(can_export(document))
                            .show(ui)
                            .clicked()
                        {
                            pressed = true;
                        }
                    });
                });
                let name = export::filename(&self.base_name, self.format, &self.configuration);
                caption(ui, style, &name);
            },
        );

        if dismissed {
            self.notice = None;
        }
        if pressed {
            self.press_export(document, files, language);
        }
        self.show_confirmation(ui, style, document, files);

        let close_requested = (response.close_requested || self.finished)
            && self.confirm.is_none()
            && !self.is_pending();
        SheetScreenResponse { close_requested }
    }

    fn show_confirmation(
        &mut self,
        ui: &mut Ui,
        style: SheetStyle,
        document: &Document,
        files: &dyn DocumentFiles,
    ) {
        let Some(configuration) = self.confirm.clone() else {
            return;
        };
        let language = style.language;
        let actions = [
            AlertAction::new(tr(language, "Export Anyway"), ButtonKind::Destructive),
            AlertAction::new(tr(language, "Cancel"), ButtonKind::Bordered),
        ];
        let pressed = modal(
            ui,
            "mobile-export-ambiguous",
            tr(language, "Export Anyway?"),
            tr(
                language,
                "Quotes are off and a header or cell contains the separator or a line break. The exported file may not read back as the same table.",
            ),
            &actions,
        );
        let Some(index) = pressed else {
            return;
        };
        self.confirm = None;
        if index == 0 {
            self.configuration = configuration;
            if let Some(table) = document.visible_table() {
                self.write_export(&table, document, files, language, true);
            }
        }
    }
}
