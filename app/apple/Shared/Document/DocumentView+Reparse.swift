import SwiftUI
import TabToolKit

/// The reparse lifecycle: applying a format change, re-reading the file, and
/// asking for confirmation when the edits would be lost.
extension DocumentView {
    /// Applies one format-bar edit through the planner.
    func applyConfiguration(_ configuration: CSVConfiguration) {
        guard !isReparsing else { return }
        switch FormatChangePlanner.plan(from: document.configuration,
                                       to: configuration) {
        case .none:
            break
        case .decimalMark(let mark):
            if !document.hasContent {
                startReparse(configuration: configuration)
            } else {
                document.setDecimalMark(mark)
            }
        case .header(let enabled):
            if !document.hasContent {
                startReparse(configuration: configuration)
            } else {
                document.setHeaderEnabled(enabled)
            }
        case .reparse(let configuration):
            startReparse(configuration: configuration)
        }
    }

    /// Re-reads the file after an explicit format change.
    ///
    /// The dirty check runs off the main actor. Only an actually dirty
    /// document asks to discard edits; a re-read rejected because it raced
    /// with another change is reported as a retry instead.
    func startReparse(configuration: CSVConfiguration) {
        guard !isReparsing else { return }
        isReparsing = true
        Task {
            await continueReparse(to: configuration)
            isReparsing = false
        }
    }

    func continueReparse(to configuration: CSVConfiguration) async {
        guard document.fileURL != nil, document.hasContent else {
            await applyReparse(to: configuration)
            return
        }
        if await document.hasUnsavedChangesAsync() {
            #if os(iOS)
            // Dismiss the format sheet first: its `onDismiss` then shows the
            // discard confirmation, so the two presentations never overlap.
            let wasShowingFormatSheet = showsFormatSheet
            showsFormatSheet = false
            #endif
            pendingReparse = configuration
            #if os(macOS)
            isConfirmingReparse = true
            #else
            if !wasShowingFormatSheet {
                isConfirmingReparse = true
            }
            #endif
            return
        }
        await applyReparse(to: configuration)
    }

    func startConfirmedReparse(_ configuration: CSVConfiguration) {
        pendingReparse = nil
        isReparsing = true
        Task {
            await applyReparse(to: configuration)
            isReparsing = false
        }
    }

    func applyReparse(to configuration: CSVConfiguration) async {
        do {
            try await document.applyParsingConfiguration(configuration, force: true)
        } catch DocumentError.requiresConfirmation {
            formatAlert = FormatAlert(
                title: "Re-read Interrupted",
                message: String(localized: "The document changed while it was being re-read, so the new format was not applied. Try again.")
            )
        } catch {
            formatAlert = FormatAlert(
                title: "Format Not Applied",
                message: FormatErrorText.alertMessage(for: error)
            )
        }
    }

    func detectAutomatically() {
        guard !isReparsing, document.fileURL != nil else { return }
        isReparsing = true
        Task {
            do {
                let detected = try await document.detectFormatFromDisk()
                if !document.hasContent || detected != document.configuration {
                    await continueReparse(to: detected)
                }
            } catch {
                formatAlert = FormatAlert(
                    title: "Format Not Detected",
                    message: FormatErrorText.alertMessage(for: error)
                )
            }
            isReparsing = false
        }
    }
}

/// The alert that explains why a format change was not applied.
struct FormatAlert: Identifiable {
    let id = UUID()
    let title: LocalizedStringKey
    let message: String
}
