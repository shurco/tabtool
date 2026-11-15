//! Export to CSV, JSON or XML with decimal-mark conversion and the CSV export
//! header rule.

use crate::{
    CsvConfiguration, CsvError, CsvTable, CsvWriteIssue, CsvWriter, column_name, issue,
    json_writer, xml_writer,
};

/// The file format an export writes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExportFormat {
    /// Delimited text that follows the export configuration.
    Csv,
    /// A UTF-8 JSON array with one object per row; every value is a string.
    Json,
    /// A UTF-8 XML document with one element per row.
    Xml,
}

impl ExportFormat {
    /// Every format, in picker order.
    pub const ALL: [Self; 3] = [Self::Csv, Self::Json, Self::Xml];

    /// The format's display name; it is not localized.
    pub fn name(self) -> &'static str {
        match self {
            Self::Csv => "CSV",
            Self::Json => "JSON",
            Self::Xml => "XML",
        }
    }
}

/// The document and export formats of one export operation.
pub struct ExportInput<'a> {
    /// The document table; export never modifies it.
    pub table: &'a CsvTable,
    /// The file format to write.
    pub format: ExportFormat,
    /// The format the document is stored in.
    pub document_configuration: &'a CsvConfiguration,
    /// The format to export with.
    pub export_configuration: &'a CsvConfiguration,
}

/// Returns the first issue [`export_data`] would fail with, or `None` when the
/// export can be written. JSON and XML are UTF-8 and escape every value, so
/// they never have an issue.
///
/// The check runs on the converted copy because decimal-mark conversion can
/// add or remove the separator inside numeric cells.
pub fn export_preflight_issue(input: &ExportInput<'_>) -> Option<CsvWriteIssue> {
    if input.format != ExportFormat::Csv {
        return None;
    }
    let converted = converted_table(input);
    let header = export_header(input);
    issue(&converted, header.as_deref(), input.export_configuration)
}

/// Converts a copy of the document to the export format and returns the
/// encoded data.
///
/// Numeric cells are copied with their decimal mark changed from the document
/// mark to the export mark; the source table is not modified.
///
/// CSV: when the export configuration enables a header, the document headers
/// are used, falling back to spreadsheet-style names (A, B, ...) for a table
/// without headers. Quoting, encoding and ambiguity checks follow the export
/// configuration.
///
/// JSON and XML: UTF-8, keyed by the headers or the generated names, made
/// unique with `_2`, `_3`, ...; only the export decimal mark applies.
pub fn export_data(
    input: &ExportInput<'_>,
    allow_ambiguous_output: bool,
) -> Result<Vec<u8>, CsvError> {
    let converted = converted_table(input);
    match input.format {
        ExportFormat::Json => return Ok(json_writer::json_string(&converted).into_bytes()),
        ExportFormat::Xml => return Ok(xml_writer::xml_string(&converted).into_bytes()),
        ExportFormat::Csv => {}
    }
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
