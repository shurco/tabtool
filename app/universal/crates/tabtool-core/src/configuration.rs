//! The CSV format used to read and write a document.

use crate::Encoding;

/// The CSV format used to read and write a document.
///
/// The defaults are UTF-8, comma separator, quotes escaped by doubling,
/// decimal point and no header row.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CsvConfiguration {
    /// Text encoding of the file.
    pub encoding: Encoding,
    /// Field separator, usually `,` or `;`.
    pub column_separator: String,
    /// Quote character, usually `"`.
    pub quote_character: String,
    /// Character that escapes a quote inside a quoted field.
    pub escape_character: String,
    /// Decimal mark used by numeric cells.
    pub decimal_mark: String,
    /// Whether the first row names the columns.
    pub first_row_as_header: bool,
}

impl Default for CsvConfiguration {
    fn default() -> Self {
        Self {
            encoding: Encoding::Utf8,
            column_separator: ",".to_string(),
            quote_character: "\"".to_string(),
            escape_character: "\"".to_string(),
            decimal_mark: ".".to_string(),
            first_row_as_header: false,
        }
    }
}
