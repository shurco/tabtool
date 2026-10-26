/// Errors produced by reading, writing, and exporting CSV data.
public enum CSVError: Error, Equatable, Sendable {
    /// The input bytes are not valid text in the selected encoding.
    case cannotDecodeData
    /// A quoted field is not closed or ends with an incomplete escape sequence.
    case malformedQuotedField
    /// A quote character appears inside an unquoted field.
    case quoteInUnquotedField
    /// The text cannot be represented in the selected encoding.
    case unencodableText
    /// A no-quote cell contains the separator or a line break.
    case ambiguousOutput
}
