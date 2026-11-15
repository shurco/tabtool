//! The Export sheet, the large-file Save As copy and the saved-copy notice.

use std::path::PathBuf;

use egui::{Align, Layout, Ui};
use tabtool_core::{CsvConfiguration, CsvError, CsvWriteIssue};
use tabtool_document::{Document, DocumentError};
use tabtool_i18n::{Language, tr};

use super::export_file::{clean_stale_exports, remove_temporary_export, temporary_export};
use super::{SheetScreenResponse, SheetStyle, caption, form_row, touch_button, touch_toggle};
use crate::document::{alert_message, can_export};
use crate::export;
use crate::export::controls::{
    PickerSize, decimal_picker, encoding_picker, quote_picker, separator_picker,
};
use crate::mobile::{DocumentFiles, ExportOutcome, SheetOptions, sheet};
use crate::widgets::{AlertAction, BannerKind, ButtonKind, banner, modal};

/// The catalog key of an export that did not finish.
const EXPORT_NOT_COMPLETED: &str = "Export Not Completed";

/// The Export sheet state.
pub struct ExportSheet {
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
                        true,
                    );
                    caption(
                        ui,
                        style,
                        tr(language, "The header is written only to the exported file."),
                    );

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
                let name = export::filename(&self.base_name, &self.configuration);
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

    fn press_export(&mut self, document: &Document, files: &dyn DocumentFiles, language: Language) {
        let Some(table) = document.visible_table() else {
            self.notice = Some(ExportNotice {
                title: EXPORT_NOT_COMPLETED,
                message: alert_message(language, DocumentError::NotReady),
            });
            return;
        };
        match export::preflight(&table, document.configuration(), &self.configuration) {
            None => {
                self.write_export(&table, document, files, language, false);
            }
            Some(CsvWriteIssue::AmbiguousOutput) => {
                self.confirm = Some(self.configuration.clone());
            }
            Some(CsvWriteIssue::UnencodableText) => {
                self.notice = Some(self.encoding_notice(language));
            }
        }
    }

    fn write_export(
        &mut self,
        table: &tabtool_core::CsvTable,
        document: &Document,
        files: &dyn DocumentFiles,
        language: Language,
        allow_ambiguous: bool,
    ) {
        match export::data(
            table,
            document.configuration(),
            &self.configuration,
            allow_ambiguous,
        ) {
            Ok(data) => self.start_copy(&data, files, language),
            Err(CsvError::AmbiguousOutput) if !allow_ambiguous => {
                self.confirm = Some(self.configuration.clone());
            }
            Err(error) => {
                self.notice = Some(ExportNotice {
                    title: EXPORT_NOT_COMPLETED,
                    message: alert_message(language, DocumentError::Csv(error)),
                });
            }
        }
    }

    fn start_copy(&mut self, data: &[u8], files: &dyn DocumentFiles, language: Language) {
        let name = export::filename(&self.base_name, &self.configuration);
        let base = files.documents_dir();
        clean_stale_exports(&base);
        let source = match temporary_export(&base, data, &name) {
            Ok(source) => source,
            Err(_) => {
                self.notice = Some(ExportNotice {
                    title: EXPORT_NOT_COMPLETED,
                    message: tr(language, "The file could not be saved").to_owned(),
                });
                return;
            }
        };
        let outcome = files.export_copy(&source, &name);
        if files.export_in_progress() {
            self.pending = Some(PendingExport { source, name });
            return;
        }
        remove_temporary_export(&source);
        self.finish(outcome, language);
    }

    fn finish(&mut self, outcome: ExportOutcome, language: Language) {
        match outcome {
            ExportOutcome::Copied | ExportOutcome::Cancelled => self.finished = true,
            ExportOutcome::Failed => {
                self.notice = Some(ExportNotice {
                    title: EXPORT_NOT_COMPLETED,
                    message: tr(language, "The file could not be saved").to_owned(),
                });
            }
        }
    }

    fn poll_pending(&mut self, files: &dyn DocumentFiles, language: Language) {
        if self.pending.is_none() || files.export_in_progress() {
            return;
        }
        let Some(pending) = self.pending.take() else {
            return;
        };
        let outcome = files.export_copy(&pending.source, &pending.name);
        remove_temporary_export(&pending.source);
        self.finish(outcome, language);
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

    fn encoding_notice(&self, language: Language) -> ExportNotice {
        ExportNotice {
            title: EXPORT_NOT_COMPLETED,
            message: alert_message(language, DocumentError::Csv(CsvError::UnencodableText)),
        }
    }
}
