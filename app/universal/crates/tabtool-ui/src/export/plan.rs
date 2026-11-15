//! The export file mapping and preflight.

use std::path::Path;

use tabtool_core::{
    CsvConfiguration, CsvError, CsvTable, CsvWriteIssue, ExportInput, export_data,
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

/// The complete suggested file name for `configuration`.
pub fn filename(base_name: &str, configuration: &CsvConfiguration) -> String {
    format!("{base_name}.{}", file_extension(configuration))
}

/// The extension an export writes: `tsv` for a tab, `csv` otherwise.
pub fn file_extension(configuration: &CsvConfiguration) -> &'static str {
    if configuration.column_separator == "\t" {
        "tsv"
    } else {
        "csv"
    }
}

/// What blocks writing `table` converted to the export configuration.
///
/// `None` writes without asking; [`CsvWriteIssue::AmbiguousOutput`] asks
/// first because quotes are off; [`CsvWriteIssue::UnencodableText`] cannot be
/// represented in the chosen encoding.
pub fn preflight(
    table: &CsvTable,
    document_configuration: &CsvConfiguration,
    export_configuration: &CsvConfiguration,
) -> Option<CsvWriteIssue> {
    export_preflight_issue(&ExportInput {
        table,
        document_configuration,
        export_configuration,
    })
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
