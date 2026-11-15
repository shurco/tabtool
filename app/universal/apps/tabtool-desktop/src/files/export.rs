//! Export sessions and their preflight, save panel and write flow.

use tabtool_core::{CsvConfiguration, CsvTable, CsvWriteIssue, ExportFormat};
use tabtool_document::{Document, DocumentError};
use tabtool_ui::export;

use super::FilePolicy;

/// The table and format captured when the export panel opened.
///
/// The table is a copy and the configuration is owned, so export never changes
/// the document and a later edit cannot leak into an open panel.
#[derive(Clone, Debug)]
pub struct ExportSession {
    /// The candidate file format shown in the panel.
    pub export_format: ExportFormat,
    /// The candidate CSV format shown in the panel.
    pub export_configuration: CsvConfiguration,
    /// The visible table at the time the panel opened.
    pub table: CsvTable,
    /// The document format the table was read with.
    pub document_configuration: CsvConfiguration,
    /// The document stem plus `-export`.
    pub base_name: String,
}

impl ExportSession {
    /// The complete file name for the current candidate.
    pub fn suggested_name(&self) -> String {
        export::filename(
            &self.base_name,
            self.export_format,
            &self.export_configuration,
        )
    }

    /// The extension the current candidate writes.
    pub fn file_extension(&self) -> &'static str {
        export::file_extension(self.export_format, &self.export_configuration)
    }
}

/// How an export request ended.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExportOutcome {
    /// The bytes are on disk.
    Saved,
    /// The save panel was cancelled.
    Cancelled,
    /// Quotes are off and the output is ambiguous; confirm first.
    NeedsConfirmation,
    /// The text cannot be represented in the chosen encoding.
    BlockedUnencodable,
    /// Serializing or writing failed; no partial file was left behind.
    Failed(DocumentError),
}

impl FilePolicy {
    /// Captures the visible table and format for the export panel.
    ///
    /// A large or not-yet-loaded document has no in-memory table, so it cannot
    /// export; the toolbar disables Export for the same state.
    pub(crate) fn export_session(&self, document: &Document) -> Option<ExportSession> {
        let table = document.visible_table()?;
        let configuration = document.configuration().clone();
        Some(ExportSession {
            export_format: ExportFormat::Csv,
            export_configuration: configuration.clone(),
            table,
            document_configuration: configuration,
            base_name: export::base_name(document.file_path()),
        })
    }

    /// Runs the candidate through preflight, the save panel and the write.
    pub(crate) fn export(&self, session: &ExportSession, allow_ambiguous: bool) -> ExportOutcome {
        match export::preflight(
            &session.table,
            session.export_format,
            &session.document_configuration,
            &session.export_configuration,
        ) {
            Some(CsvWriteIssue::UnencodableText) => return ExportOutcome::BlockedUnencodable,
            Some(CsvWriteIssue::AmbiguousOutput) if !allow_ambiguous => {
                return ExportOutcome::NeedsConfirmation;
            }
            None | Some(CsvWriteIssue::AmbiguousOutput) => {}
        }

        let Some(path) = self
            .dialogs
            .save_file(&session.suggested_name(), session.file_extension())
        else {
            return ExportOutcome::Cancelled;
        };
        match export::data(
            &session.table,
            session.export_format,
            &session.document_configuration,
            &session.export_configuration,
            allow_ambiguous,
        ) {
            Ok(data) => match self.storage.write_atomic(&path, &data) {
                Ok(()) => ExportOutcome::Saved,
                Err(_) => ExportOutcome::Failed(DocumentError::CannotWriteFile),
            },
            Err(error) => ExportOutcome::Failed(DocumentError::Csv(error)),
        }
    }
}
