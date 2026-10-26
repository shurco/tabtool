import SwiftUI
import TabToolKit

/// The full-area state shown when the initial read failed.
///
/// The format bar stays available underneath, so the user can change the
/// encoding or other parsing settings and retry the read.
struct ErrorOverlay: View {
    let phase: DocumentPhase
    let canDetect: Bool
    let onDetectAutomatically: () -> Void

    var body: some View {
        VStack(spacing: 12) {
            Image(systemName: "exclamationmark.triangle")
                .font(.system(size: 38, weight: .regular))
                .foregroundStyle(.secondary)
                .accessibilityHidden(true)

            Text(FormatErrorText.title(for: phase))
                .font(.title2.weight(.semibold))
                .multilineTextAlignment(.center)

            Text(FormatErrorText.message(for: phase))
                .multilineTextAlignment(.center)
                .frame(maxWidth: 460)

            if let hint = FormatErrorText.hint(for: phase) {
                Text(hint)
                    .font(.callout)
                    .foregroundStyle(.secondary)
                    .multilineTextAlignment(.center)
                    .frame(maxWidth: 460)
            }

            Button("Detect Automatically", action: onDetectAutomatically)
                .buttonStyle(.borderedProminent)
                .disabled(!canDetect)
                .accessibilityIdentifier("detect-format")
                .padding(.top, 4)
        }
        .padding(32)
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .background(.background)
        .accessibilityElement(children: .contain)
        .accessibilityIdentifier("error-overlay")
    }
}

/// User-facing text for read and format-apply failures.
enum FormatErrorText {
    static func title(for phase: DocumentPhase) -> String {
        switch phase {
        case .readFailed:
            return "The file could not be read"
        case .unreadableFile:
            return "The file could not be opened"
        default:
            return "The document is not available"
        }
    }

    static func message(for phase: DocumentPhase) -> String {
        switch phase {
        case .readFailed(let error):
            return message(for: error)
        case .unreadableFile:
            return "The file could not be read from disk."
        default:
            return "The document is not available."
        }
    }

    static func hint(for phase: DocumentPhase) -> String? {
        switch phase {
        case .readFailed(let error):
            return hint(for: error)
        case .unreadableFile:
            return "Check that the file still exists and that you have permission to open it."
        default:
            return nil
        }
    }

    /// A single alert body with the failure and its recovery suggestion.
    static func alertMessage(for error: Error) -> String {
        let recovery = hint(for: error).map { "\n\n\($0)" } ?? ""
        return message(for: error) + recovery
    }

    private static func message(for error: Error) -> String {
        if let csvError = error as? CSVError {
            return message(for: csvError)
        }
        if let documentError = error as? DocumentError {
            return message(for: documentError)
        }
        return error.localizedDescription
    }

    private static func hint(for error: Error) -> String? {
        if let csvError = error as? CSVError {
            return hint(for: csvError)
        }
        if error as? DocumentError == .cannotReadFile {
            return "Check that the file still exists and that you have permission to open it."
        }
        return nil
    }

    private static func message(for error: CSVError) -> String {
        switch error {
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
        }
    }

    private static func message(for error: DocumentError) -> String {
        switch error {
        case .notReady:
            return "The document is still loading or could not be read."
        case .requiresConfirmation:
            return "The document changed while the operation was running. Try again."
        case .cannotReadFile:
            return "The file could not be read from disk."
        }
    }

    private static func hint(for error: CSVError) -> String? {
        switch error {
        case .cannotDecodeData:
            return "Choose another encoding, or detect the format automatically."
        case .malformedQuotedField, .quoteInUnquotedField:
            return "Check the quote style and escape character, or detect the format automatically."
        case .unencodableText:
            return "Choose an encoding that can represent the text."
        case .ambiguousOutput:
            return "Turn quotes on, or remove separators and line breaks from cells."
        }
    }
}
