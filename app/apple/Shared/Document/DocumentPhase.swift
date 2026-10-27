import Foundation
import TabToolKit

/// Why the document cannot currently be edited or saved.
enum DocumentPhase: Equatable {
    /// Bytes are read, format detection and parsing are still running.
    case loading
    /// The table can be displayed, edited, and saved.
    case ready
    /// Parsing the bytes failed; the format must be fixed before retrying.
    case readFailed(CSVError)
    /// The file bytes could not be read at all, as distinct from a parse error.
    case unreadableFile
    /// The table is loaded but cannot be written safely with its configuration.
    case needsAttention(CSVWriteIssue)
}

/// Document operations that fail for reasons unrelated to CSV parsing.
enum DocumentError: Error, Equatable {
    /// The table is not loaded yet or the file could not be read.
    case notReady
    /// The operation was rejected because it would discard unsaved edits, or
    /// because it raced with another change.
    ///
    /// The format UI checks `hasUnsavedChangesAsync()` before forcing a
    /// re-read: a dirty document gets a discard prompt, while a rejected
    /// re-read gets a retry message.
    case requiresConfirmation
    /// The file bytes are unavailable to the document.
    case cannotReadFile
}

/// Localized fallback text for generic error presentation.
extension DocumentError: LocalizedError {
    var errorDescription: String? {
        switch self {
        case .notReady:
            return String(localized: "The document is still loading or could not be read.")
        case .requiresConfirmation:
            return String(localized: "The document changed while the operation was running. Try again.")
        case .cannotReadFile:
            return String(localized: "The file could not be read from disk.")
        }
    }
}
