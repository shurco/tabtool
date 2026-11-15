//! Writing the export: preflight, the temporary copy and the platform picker.

use tabtool_core::{CsvError, CsvWriteIssue};
use tabtool_document::{Document, DocumentError};
use tabtool_i18n::{Language, tr};

use super::super::export_file::{clean_stale_exports, remove_temporary_export, temporary_export};
use super::{EXPORT_NOT_COMPLETED, ExportNotice, ExportSheet, PendingExport};
use crate::document::alert_message;
use crate::export;
use crate::mobile::{DocumentFiles, ExportOutcome};

impl ExportSheet {
    pub(super) fn press_export(
        &mut self,
        document: &Document,
        files: &dyn DocumentFiles,
        language: Language,
    ) {
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

    pub(super) fn write_export(
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

    pub(super) fn poll_pending(&mut self, files: &dyn DocumentFiles, language: Language) {
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

    fn encoding_notice(&self, language: Language) -> ExportNotice {
        ExportNotice {
            title: EXPORT_NOT_COMPLETED,
            message: alert_message(language, DocumentError::Csv(CsvError::UnencodableText)),
        }
    }
}
