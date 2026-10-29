import UIKit

extension RevertCoordinator {
    /// Runs the revert flow for the focused document.
    static func present(for document: CSVDocument) {
        Task {
            await run(for: document)
        }
    }

    /// Asks whether unsaved edits may be discarded.
    static func confirmDiscard(for _: CSVDocument) async -> Bool {
        await withCheckedContinuation { continuation in
            let alert = UIAlertController(
                title: String(localized: "Revert to Saved?"),
                message: String(localized: "Your unsaved changes will be discarded. The table will be replaced with the last saved version from disk."),
                preferredStyle: .alert
            )
            var resumed = false
            let finish: (Bool) -> Void = { value in
                guard !resumed else { return }
                resumed = true
                continuation.resume(returning: value)
            }
            alert.addAction(UIAlertAction(title: String(localized: "Revert"), style: .destructive) { _ in
                finish(true)
            })
            alert.addAction(UIAlertAction(title: String(localized: "Cancel"), style: .cancel) { _ in
                finish(false)
            })
            if !KeyWindow.present(alert) {
                finish(false)
            }
        }
    }

    static func presentError(_ error: Error) {
        let alert = UIAlertController(
            title: String(localized: "Revert Not Completed"),
            message: FormatErrorText.alertMessage(for: error),
            preferredStyle: .alert
        )
        alert.addAction(UIAlertAction(title: String(localized: "OK"), style: .default))
        _ = KeyWindow.present(alert)
    }
}
