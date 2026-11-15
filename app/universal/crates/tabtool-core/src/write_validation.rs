//! Checks whether a table can be saved without losing its structure or text.

use crate::parser::is_row_break;
use crate::{CsvConfiguration, CsvTable, CsvWriter, Encoding};

/// A problem that prevents a table from being written as CSV.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CsvWriteIssue {
    /// A header or cell cannot be represented in the configured encoding.
    UnencodableText,
    /// With quoting disabled, a header or cell would change the table
    /// structure: it contains the column separator or a line break, or the
    /// output would start with a literal U+FEFF byte order mark.
    AmbiguousOutput,
}

/// Returns the first blocking issue, or `None` when the table can be saved.
///
/// The checks run in the same order as [`CsvWriter::data`]: ambiguity first,
/// then encodability of the exact text that saving would produce.
pub fn issue(
    table: &CsvTable,
    header: Option<&[String]>,
    configuration: &CsvConfiguration,
) -> Option<CsvWriteIssue> {
    if is_ambiguous(table, header, configuration) {
        return Some(CsvWriteIssue::AmbiguousOutput);
    }
    // Every Rust string is representable in UTF-8, which is the default and
    // keeps validation constant-time for ordinary documents.
    if configuration.encoding == Encoding::Utf8 {
        return None;
    }

    let text = CsvWriter::serialized_text(table, header, configuration);
    configuration
        .encoding
        .encode(&text)
        .err()
        .map(|_| CsvWriteIssue::UnencodableText)
}

/// Whether unquoted output would parse as a different table.
///
/// Besides separators and line breaks, the first field is checked for a
/// leading U+FEFF: [`crate::CsvParser`] removes a leading UTF-8 byte order
/// mark, so a no-quote file whose first character is a literal U+FEFF would
/// lose that character when the app reads it back. Quoted output begins with
/// the quote character, so it cannot collide with the BOM.
pub fn is_ambiguous(
    table: &CsvTable,
    header: Option<&[String]>,
    configuration: &CsvConfiguration,
) -> bool {
    if !configuration.quote_character.is_empty() {
        return false;
    }

    // Only the first field of the output can become a byte order mark. When a
    // header is written it is the first field; otherwise it is the first cell
    // of the first column. An empty header row writes no fields, so it cannot
    // start with a BOM either.
    let first_field = match header {
        Some(fields) => fields.first(),
        None => table
            .columns
            .first()
            .and_then(|column| column.cells.first()),
    };
    if first_field.is_some_and(|field| field.starts_with('\u{FEFF}')) {
        return true;
    }

    // A final empty single-column row contributes no characters after the
    // preceding row separator, so the output ends with LF and reparsing drops
    // that row. A lone empty cell is fine: empty output parses back to one
    // empty row.
    if table.columns.len() == 1
        && table
            .columns
            .first()
            .and_then(|column| column.cells.last())
            .is_some_and(String::is_empty)
        && (header.is_some() || table.row_count() > 1)
    {
        return true;
    }

    if header.is_some_and(|header| {
        header
            .iter()
            .any(|field| is_field_ambiguous(field, configuration))
    }) {
        return true;
    }
    table
        .columns
        .iter()
        .flat_map(|column| &column.cells)
        .any(|cell| is_field_ambiguous(cell, configuration))
}

/// Whether an unquoted field contains the separator or a row break.
fn is_field_ambiguous(field: &str, configuration: &CsvConfiguration) -> bool {
    // A cheap byte scan clears almost every field: the first byte of the
    // separator and the lead bytes of LF, CR, NEL, LS and PS are the only ways
    // a field can contain one of them.
    let separator_byte = configuration.column_separator.as_bytes().first().copied();
    let may_break = field
        .as_bytes()
        .iter()
        .any(|&byte| separator_byte == Some(byte) || matches!(byte, 0x0A | 0x0D | 0xC2 | 0xE2));
    if !may_break {
        return false;
    }

    let separator = configuration.column_separator.as_str();
    if !separator.is_empty() && field.contains(separator) {
        return true;
    }
    field.chars().any(is_row_break)
}
