//! The export file mapping and preflight.

use std::path::Path;

use tabtool_core::{
    CsvConfiguration, CsvError, CsvTable, CsvWriteIssue, ExportFormat, ExportInput, export_data,
    export_preflight_issue,
};

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

/// The complete suggested file name for `format` and `configuration`.
pub fn filename(base_name: &str, format: ExportFormat, configuration: &CsvConfiguration) -> String {
    format!("{base_name}.{}", file_extension(format, configuration))
}

/// The extension an export writes: `json` or `xml` for those formats; for CSV,
/// `tsv` for a tab separator and `csv` otherwise.
pub fn file_extension(format: ExportFormat, configuration: &CsvConfiguration) -> &'static str {
    match format {
        ExportFormat::Json => "json",
        ExportFormat::Xml => "xml",
        ExportFormat::Csv if configuration.column_separator == "\t" => "tsv",
        ExportFormat::Csv => "csv",
    }
}

/// What blocks writing `table` converted to the export configuration.
///
/// `None` writes without asking; [`CsvWriteIssue::AmbiguousOutput`] asks
/// first because quotes are off; [`CsvWriteIssue::UnencodableText`] cannot be
/// represented in the chosen encoding. JSON and XML always return `None`.
pub fn preflight(
    table: &CsvTable,
    format: ExportFormat,
    document_configuration: &CsvConfiguration,
    export_configuration: &CsvConfiguration,
) -> Option<CsvWriteIssue> {
    export_preflight_issue(&ExportInput {
        table,
        format,
        document_configuration,
        export_configuration,
    })
}

/// Serializes a converted copy of `table`; the source is never modified.
pub fn data(
    table: &CsvTable,
    format: ExportFormat,
    document_configuration: &CsvConfiguration,
    export_configuration: &CsvConfiguration,
    allow_ambiguous_output: bool,
) -> Result<Vec<u8>, CsvError> {
    export_data(
        &ExportInput {
            table,
            format,
            document_configuration,
            export_configuration,
        },
        allow_ambiguous_output,
    )
}
