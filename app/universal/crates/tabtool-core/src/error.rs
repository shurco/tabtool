//! Typed failures produced by reading, writing and exporting CSV data.

/// Errors produced by reading, writing and exporting CSV data.
///
/// The display text belongs to the UI layer; this type carries only the case.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CsvError {
    /// The input bytes are not valid text in the selected encoding.
    CannotDecodeData,
    /// A quoted field is not closed or ends with an incomplete escape sequence.
    MalformedQuotedField,
    /// A quote character appears inside an unquoted field.
    QuoteInUnquotedField,
    /// The text cannot be represented in the selected encoding.
    UnencodableText,
    /// A no-quote cell contains the separator or a line break.
    AmbiguousOutput,
    /// The file uses an encoding that large file mode cannot scan by bytes.
    UnsupportedLargeFileEncoding,
    /// The separator, quote or escape is not a single ASCII character, which
    /// large file mode needs to find rows by bytes.
    UnsupportedLargeFileDelimiter,
    /// The row cannot be edited now: indexing is unfinished, the data is
    /// unreadable, or the file changed on disk.
    RowNotEditable,
}
