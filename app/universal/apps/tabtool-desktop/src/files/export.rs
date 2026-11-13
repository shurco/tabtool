//! Export sessions and their preflight, save panel and write flow.

use tabtool_core::{CsvConfiguration, CsvTable};
use tabtool_document::{Document, DocumentError, Storage};
use tabtool_platform::FileDialogs;
use tabtool_ui::export::{ExportDecision, ExportOptions, ExportPlan};

/// The table and format captured when the export panel opened.
///
/// The table is a copy and the configuration is owned, so export never changes
/// the document and a later edit cannot leak into an open panel.
#[derive(Clone, Debug)]
pub struct ExportSession {
    /// The candidate format shown in the panel.
    pub options: ExportOptions,
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
        self.options.suggested_name(&self.base_name)
    }

    /// The extension the current candidate writes.
    pub fn file_extension(&self) -> &'static str {
        self.options.file_extension()
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

/// Captures the visible table and format of `document`.
///
/// A large or not-yet-loaded document has no in-memory table, so it cannot
/// export; the toolbar disables Export for the same state.
pub fn export_session(document: &Document) -> Option<ExportSession> {
    let table = document.visible_table()?;
    let configuration = document.configuration().clone();
    Some(ExportSession {
        options: ExportOptions::new(configuration.clone()),
        table,
        document_configuration: configuration,
        base_name: ExportPlan::base_name(document.file_path()),
    })
}

/// Runs preflight and, when allowed, the save panel and the atomic write.
pub fn run_export(
    session: &ExportSession,
    allow_ambiguous: bool,
    dialogs: &dyn FileDialogs,
    storage: &dyn Storage,
) -> ExportOutcome {
    match ExportPlan::decision(
        &session.table,
        &session.document_configuration,
        &session.options.configuration,
    ) {
        ExportDecision::BlockedUnencodable => return ExportOutcome::BlockedUnencodable,
        ExportDecision::ConfirmAmbiguous if !allow_ambiguous => {
            return ExportOutcome::NeedsConfirmation;
        }
        ExportDecision::Proceed | ExportDecision::ConfirmAmbiguous => {}
    }

    let Some(path) = dialogs.save_file(&session.suggested_name(), session.file_extension()) else {
        return ExportOutcome::Cancelled;
    };
    match ExportPlan::data(
        &session.table,
        &session.document_configuration,
        &session.options.configuration,
        allow_ambiguous,
    ) {
        Ok(data) => match storage.write_atomic(&path, &data) {
            Ok(()) => ExportOutcome::Saved,
            Err(_) => ExportOutcome::Failed(DocumentError::CannotWriteFile),
        },
        Err(error) => ExportOutcome::Failed(DocumentError::Csv(error)),
    }
}
