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
            if !present(alert) {
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
        _ = present(alert)
    }

    /// Presents `alert` on the topmost view controller. Returns false when no
    /// window is available, so the caller can resume a continuation.
    @discardableResult
    private static func present(_ alert: UIAlertController) -> Bool {
        let scenes = UIApplication.shared.connectedScenes.compactMap { $0 as? UIWindowScene }
        guard let root = scenes.flatMap(\.windows).first(where: \.isKeyWindow)?.rootViewController
            ?? scenes.first?.windows.first?.rootViewController else {
            return false
        }
        var presenter = root
        while let presented = presenter.presentedViewController {
            presenter = presented
        }
        presenter.present(alert, animated: true)
        return true
    }
}
