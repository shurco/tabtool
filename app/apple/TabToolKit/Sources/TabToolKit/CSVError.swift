import Foundation

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
    /// The row cannot be edited now: indexing is unfinished, the data is
    /// unreadable, or the file changed on disk.
    case rowNotEditable
}

/// English fallback text for generic error presentation.
///
/// The app's `FormatErrorText` provides localized, context-aware messages for
/// the same cases; this conformance keeps framework alerts (including save and
/// autosave failures) useful instead of showing a generic serialization error.
extension CSVError: LocalizedError {
    public var errorDescription: String? {
        switch self {
        case .cannotDecodeData:
            return "The file is not valid text in the selected encoding."
        case .malformedQuotedField:
            return "A quoted field is not closed or ends with an incomplete escape."
        case .quoteInUnquotedField:
            return "A quote character appears inside an unquoted field."
        case .unencodableText:
            return "The text cannot be represented in the selected encoding."
        case .ambiguousOutput:
            return "A cell contains the separator or a line break, but quotes are off."
        case .rowNotEditable:
            return "This row cannot be edited right now."
        }
    }

    public var recoverySuggestion: String? {
        switch self {
        case .cannotDecodeData:
            return "Choose another encoding, or detect the format automatically."
        case .malformedQuotedField, .quoteInUnquotedField:
            return "Check the quote style and escape character, or detect the format automatically."
        case .unencodableText:
            return "Undo the edit that added the text, or export with an encoding that can represent it."
        case .ambiguousOutput:
            return "Undo the edit that added the separator or line break, or export with quotes on."
        case .rowNotEditable:
            return "Wait for indexing to finish, or reload the file if it changed on disk."
        }
    }
}
