//! Converts between table cells and tab-separated clipboard text.

use crate::localized;

/// Parses tab-separated text without quote handling.
///
/// CRLF, CR and LF each break rows. Empty fields and blank rows are preserved.
/// A single trailing row break does not create an extra row; any further
/// trailing breaks remain as empty rows.
pub fn parse_tsv(text: &str) -> Vec<Vec<String>> {
    let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
    let mut rows: Vec<&str> = normalized.split('\n').collect();
    if normalized.ends_with('\n') {
        rows.pop();
    }
    rows.into_iter()
        .map(|row| row.split('\t').map(str::to_owned).collect())
        .collect()
}

/// Produces tab-separated text with rows joined by line feeds.
///
/// Tabs and line breaks inside cells become spaces so they cannot break the
/// clipboard grid. Numeric cells are localized from `numeric_decimal_mark` to
/// `decimal_separator`. Rows are joined without a separator after the last
/// row, so the output has no trailing newline.
pub fn make_tsv(
    rows: &[Vec<String>],
    numeric_decimal_mark: &str,
    decimal_separator: &str,
) -> String {
    rows.iter()
        .map(|row| {
            row.iter()
                .map(|cell| {
                    let flattened = cell.replace("\r\n", " ").replace(['\r', '\n', '\t'], " ");
                    localized(&flattened, numeric_decimal_mark, decimal_separator)
                })
                .collect::<Vec<_>>()
                .join("\t")
        })
        .collect::<Vec<_>>()
        .join("\n")
}
