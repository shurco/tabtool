//! CSV export with decimal-mark conversion and the export header rule.

use crate::{CsvConfiguration, CsvError, CsvTable, CsvWriteIssue, CsvWriter, column_name, issue};

/// The document and export formats of one export operation.
pub struct ExportInput<'a> {
    /// The document table; export never modifies it.
    pub table: &'a CsvTable,
    /// The format the document is stored in.
    pub document_configuration: &'a CsvConfiguration,
    /// The format to export with.
    pub export_configuration: &'a CsvConfiguration,
}

/// Returns the first issue [`export_data`] would fail with, or `None` when the
/// export can be written.
///
/// The check runs on the converted copy because decimal-mark conversion can
/// add or remove the separator inside numeric cells.
pub fn export_preflight_issue(input: &ExportInput<'_>) -> Option<CsvWriteIssue> {
    let converted = converted_table(input);
    let header = export_header(input);
    issue(&converted, header.as_deref(), input.export_configuration)
}

/// Converts a copy of the document to the export format and returns the
/// encoded data.
///
/// Numeric cells are copied with their decimal mark changed from the document
/// mark to the export mark; the source table is not modified. When the export
/// configuration enables a header, the document headers are used, falling back
/// to spreadsheet-style names (A, B, ...) for a table without headers.
/// Quoting, encoding and ambiguity checks follow the export configuration.
pub fn export_data(
    input: &ExportInput<'_>,
    allow_ambiguous_output: bool,
) -> Result<Vec<u8>, CsvError> {
    let converted = converted_table(input);
    let header = export_header(input);
    let mut writer = CsvWriter::new(&converted, header.as_deref(), input.export_configuration);
    writer.allow_ambiguous_output = allow_ambiguous_output;
    writer.data()
}

/// The table the writer receives: the source with decimal marks converted.
fn converted_table(input: &ExportInput<'_>) -> CsvTable {
    let mut converted = input.table.clone();
    converted.convert_decimal_mark(
        &input.document_configuration.decimal_mark,
        &input.export_configuration.decimal_mark,
    );
    converted
}

/// The explicit header row an export writes, or `None`.
fn export_header(input: &ExportInput<'_>) -> Option<Vec<String>> {
    if !input.export_configuration.first_row_as_header {
        return None;
    }
    Some(
        input
            .table
            .headers()
            .unwrap_or_else(|| (0..input.table.columns.len()).map(column_name).collect()),
    )
}
