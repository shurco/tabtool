import AppKit

extension RevertCoordinator {
    /// Runs the revert flow for the focused document.
    static func present(for document: CSVDocument) {
        Task {
            await run(for: document)
        }
    }

    /// Asks whether unsaved edits may be discarded.
    static func confirmDiscard(for _: CSVDocument) async -> Bool {
        let alert = NSAlert()
        alert.alertStyle = .warning
        alert.messageText = String(localized: "Revert to Saved?")
        alert.informativeText = String(localized: "Your unsaved changes will be discarded. The table will be replaced with the last saved version from disk.")
        alert.addButton(withTitle: String(localized: "Revert"))
        alert.addButton(withTitle: String(localized: "Cancel"))
        return alert.runModal() == .alertFirstButtonReturn
    }

    static func presentError(_ error: Error) {
        let alert = NSAlert()
        alert.alertStyle = .warning
        alert.messageText = String(localized: "Revert Not Completed")
        alert.informativeText = FormatErrorText.alertMessage(for: error)
        alert.addButton(withTitle: String(localized: "OK"))
        alert.runModal()
    }
}
