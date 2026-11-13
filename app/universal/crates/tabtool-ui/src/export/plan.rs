//! The export file mapping and preflight decisions.

use std::path::Path;

use tabtool_core::{
    CsvConfiguration, CsvError, CsvTable, CsvWriteIssue, ExportInput, export_data,
    export_preflight_issue,
};

/// What export can do for the converted output.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExportDecision {
    /// Write the file without asking.
    Proceed,
    /// Writing with quotes off is ambiguous; ask first.
    ConfirmAmbiguous,
    /// The text cannot be represented in the chosen encoding.
    BlockedUnencodable,
}

/// The file mapping and validation rules for one export.
pub enum ExportPlan {}

impl ExportPlan {
    /// The name stem the save panel offers.
    ///
    /// The document name without its extension plus `-export`; an untitled
    /// document uses `Untitled-export`.
    pub fn base_name(document_path: Option<&Path>) -> String {
        let name = document_path
            .and_then(Path::file_stem)
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        let name = if name.is_empty() { "Untitled" } else { &name };
        format!("{name}-export")
    }

    /// The complete suggested file name for `configuration`.
    pub fn filename(base_name: &str, configuration: &CsvConfiguration) -> String {
        format!("{base_name}.{}", Self::file_extension(configuration))
    }

    /// The extension an export writes: `tsv` for a tab, `csv` otherwise.
    pub fn file_extension(configuration: &CsvConfiguration) -> &'static str {
        if configuration.column_separator == "\t" {
            "tsv"
        } else {
            "csv"
        }
    }

    /// Replaces the extension of `filename` while keeping its stem.
    pub fn renaming(filename: &str, configuration: &CsvConfiguration) -> String {
        let stem = Path::new(filename)
            .file_stem()
            .map(|stem| stem.to_string_lossy().into_owned())
            .unwrap_or_default();
        if stem.is_empty() {
            filename.to_owned()
        } else {
            format!("{stem}.{}", Self::file_extension(configuration))
        }
    }

    /// What the UI does for `table` converted to the export configuration.
    pub fn decision(
        table: &CsvTable,
        document_configuration: &CsvConfiguration,
        export_configuration: &CsvConfiguration,
    ) -> ExportDecision {
        Self::decision_for_issue(export_preflight_issue(&ExportInput {
            table,
            document_configuration,
            export_configuration,
        }))
    }

    /// What the UI does for a preflight result.
    pub fn decision_for_issue(issue: Option<CsvWriteIssue>) -> ExportDecision {
        match issue {
            None => ExportDecision::Proceed,
            Some(CsvWriteIssue::AmbiguousOutput) => ExportDecision::ConfirmAmbiguous,
            Some(CsvWriteIssue::UnencodableText) => ExportDecision::BlockedUnencodable,
        }
    }

    /// Serializes a converted copy of `table`; the source is never modified.
    pub fn data(
        table: &CsvTable,
        document_configuration: &CsvConfiguration,
        export_configuration: &CsvConfiguration,
        allow_ambiguous_output: bool,
    ) -> Result<Vec<u8>, CsvError> {
        export_data(
            &ExportInput {
                table,
                document_configuration,
                export_configuration,
            },
            allow_ambiguous_output,
        )
    }
}
