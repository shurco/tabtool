/// File > Revert to Saved, including the dirty confirmation and errors.
///
/// The flow is a small testable function: the document decides whether it is
/// dirty, the injected confirmation decides whether to discard edits, and the
/// document performs the transactional re-read and undo registration.
/// The platform file supplies `present`, `confirmDiscard`, and `presentError`.
@MainActor
enum RevertCoordinator {
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
}
