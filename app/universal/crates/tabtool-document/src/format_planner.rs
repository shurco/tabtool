//! Maps a configuration edit to the smallest operation that applies it.
//!
//! Parsing fields win over the in-memory transforms: a new encoding,
//! separator, quote character or escape character changes how the bytes are
//! read, so it must re-read them. The format bar changes one control at a
//! time; the ordering only matters when a caller substitutes several fields
//! at once.

use tabtool_core::CsvConfiguration;

/// The operation a format-bar edit needs, derived from a configuration diff.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FormatChange {
    /// The configuration did not change.
    None,
    /// Convert numeric cells in memory from the old mark to the new one.
    DecimalMark(String),
    /// Move the first row across the header boundary without re-reading.
    Header(bool),
    /// Re-read the file with the new configuration.
    Reparse(CsvConfiguration),
}

/// Maps `from` to `to` and returns the operation that applies the change.
///
/// A parsing field change plans a re-read; otherwise a decimal-mark change
/// comes before a header change, exactly like `FormatChangePlanner.swift`.
pub fn plan_format_change(from: &CsvConfiguration, to: &CsvConfiguration) -> FormatChange {
    if from == to {
        return FormatChange::None;
    }
    if from.encoding != to.encoding
        || from.column_separator != to.column_separator
        || from.quote_character != to.quote_character
        || from.escape_character != to.escape_character
    {
        return FormatChange::Reparse(to.clone());
    }
    if from.decimal_mark != to.decimal_mark {
        return FormatChange::DecimalMark(to.decimal_mark.clone());
    }
    if from.first_row_as_header != to.first_row_as_header {
        return FormatChange::Header(to.first_row_as_header);
    }
    FormatChange::None
}
