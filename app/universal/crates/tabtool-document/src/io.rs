//! Format detection and parsing for a document load.
//!
//! The document layer parses bytes in one place so loading, re-reading and
//! tests share the same header extraction and byte-order-mark bookkeeping.

use tabtool_core::{CsvConfiguration, CsvError, CsvFormatDetector, CsvParser, CsvTable};

/// A successfully parsed table together with the format and BOM state of its
/// source bytes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LoadOutcome {
    /// The logical table, with the first row moved into headers when the
    /// configuration enables header mode.
    pub table: CsvTable,
    /// The format the bytes were parsed with.
    pub configuration: CsvConfiguration,
    /// Whether the bytes began with a UTF-8 byte order mark.
    pub had_utf8_bom: bool,
}

/// Picks the most likely configuration for `bytes`.
///
/// Detection mirrors opening a file with no supplied configuration in the
/// Swift document.
pub(crate) fn detect(bytes: &[u8], prefers_chinese_encoding: bool) -> CsvConfiguration {
    CsvFormatDetector::new(bytes, prefers_chinese_encoding).detect()
}

/// Parses `bytes` with an explicit configuration.
///
/// A leading UTF-8 byte order mark is stripped by the parser and reported in
/// the outcome; other encodings handle their own byte order marks.
pub(crate) fn parse(
    bytes: &[u8],
    configuration: &CsvConfiguration,
) -> Result<LoadOutcome, CsvError> {
    let mut parser = CsvParser::from_bytes(bytes, configuration)?;
    let rows = parser.read_all()?;
    let headers = if configuration.first_row_as_header {
        rows.first().cloned()
    } else {
        None
    };
    let data_rows = rows
        .into_iter()
        .skip(usize::from(configuration.first_row_as_header))
        .collect();

    Ok(LoadOutcome {
        table: CsvTable::from_rows(data_rows, headers, 1),
        configuration: configuration.clone(),
        had_utf8_bom: parser.had_utf8_bom(),
    })
}
