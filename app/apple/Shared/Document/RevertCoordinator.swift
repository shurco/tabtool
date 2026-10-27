import AppKit

/// File > Revert to Saved, including the dirty confirmation and errors.
///
/// The flow is a small testable function: the document decides whether it is
/// dirty, the injected confirmation decides whether to discard edits, and the
/// document performs the transactional re-read and undo registration.
@MainActor
enum RevertCoordinator {
    /// Runs the revert flow for the focused document.
    static func present(for document: CSVDocument) {
        Task {
            await run(for: document)
        }
    }

    /// Runs the revert flow.
    ///
    /// Tests inject `confirm` to avoid a modal alert; production uses the
    /// standard discard confirmation.
    static func run(for document: CSVDocument,
                    confirm: (@MainActor (CSVDocument) async -> Bool)? = nil) async {
        guard document.canRevertToSaved else { return }

        if await document.hasUnsavedChangesAsync() {
            let confirmed = await (confirm ?? confirmDiscard)(document)
            guard confirmed else { return }
        }

        do {
            try await document.revertToSaved()
        } catch {
            presentError(error)
        }
    }

    /// Asks whether unsaved edits may be discarded.
    private static func confirmDiscard(for _: CSVDocument) async -> Bool {
        let alert = NSAlert()
        alert.alertStyle = .warning
        alert.messageText = String(localized: "Revert to Saved?")
        alert.informativeText = String(localized: "Your unsaved changes will be discarded. The table will be replaced with the last saved version from disk.")
        alert.addButton(withTitle: String(localized: "Revert"))
        alert.addButton(withTitle: String(localized: "Cancel"))
        return alert.runModal() == .alertFirstButtonReturn
    }

    private static func presentError(_ error: Error) {
        let alert = NSAlert()
        alert.alertStyle = .warning
        alert.messageText = String(localized: "Revert Not Completed")
        alert.informativeText = FormatErrorText.alertMessage(for: error)
        alert.addButton(withTitle: String(localized: "OK"))
        alert.runModal()
    }
}
