import Foundation
import os
import TabToolKit

/// Saving a large file: a streaming rewrite, then the written file replaces
/// the mapping on screen.
///
/// The system never saves a large document (its edits are not on the window's
/// undo manager, so it is never dirty for the system); `LargeDocumentWindow`
/// calls this for Save, Save As and the close and quit prompts.
extension CSVDocument {
    /// Writes the edits into the file, or into `destination` for Save As.
    ///
    /// Edits and undo wait until the save is over. The written file is mapped
    /// and indexed before it replaces the old mapping on screen, so the scroll
    /// position, the selection and the columns stay. A plain save without
    /// edits does nothing. On failure or `cancelLargeSave()` the file and the
    /// edits are unchanged. Throws `LargeFileSaveError`, `CancellationError`
    /// or a write error.
    func saveLargeFile(to destination: URL? = nil) async throws {
        guard let source = largeSource, let target = destination ?? fileURL else {
            throw DocumentError.notReady
        }
        endEditing()
        switch largeStatus {
        case .ready: break
        case .stale: throw LargeFileSaveError.sourceChanged
        case .none, .indexing, .saving: throw LargeFileSaveError.indexingNotFinished
        }
        guard destination != nil || hasUnsavedLargeEdits else { return }

        let save = Task { try await streamLargeSave(of: source, to: target) }
        largeSave = save
        defer { largeSave = nil }
        try await save.value
    }

    /// Stops the save in flight. Once the file is replaced the save completes.
    func cancelLargeSave() {
        largeSave?.cancel()
    }

    /// Waits until the save in flight, if any, is over. Whoever started it
    /// reports its outcome.
    func waitForLargeSave() async {
        _ = try? await largeSave?.value
    }

    private func streamLargeSave(of source: LargeCSVSource, to target: URL) async throws {
        let snapshot = source.saveSnapshot()
        let configuration = configuration
        let progress = OSAllocatedUnfairLock(initialState: 0.0)
        setLargeStatus(.saving(0))
        let poll = Task { [weak self] in
            while !Task.isCancelled {
                self?.setLargeStatus(.saving(progress.withLock { $0 }))
                try? await Task.sleep(for: .milliseconds(250))
            }
        }
        defer { poll.cancel() }

        let work = Task.detached(priority: .userInitiated) {
            try LargeFileSaver.save(snapshot, replacing: target) { value in
                progress.withLock { $0 = value }
            }
            return try Self.indexedFile(at: target, configuration: configuration)
        }
        do {
            let (file, indexer) = try await withTaskCancellationHandler {
                try await work.value
            } onCancel: {
                work.cancel()
            }
            adopt(file, indexer: indexer, at: target, replacing: source, saved: snapshot.edits)
        } catch {
            setLargeStatus(.ready)
            throw error
        }
    }

    private nonisolated static func indexedFile(at url: URL, configuration: CSVConfiguration) throws
        -> (MappedFile, LargeFileIndexer) {
        let file = try MappedFile(url: url)
        let indexer = LargeFileIndexer(file: file, configuration: configuration)
        indexer.start { _ in }
        indexer.wait()
        return (file, indexer)
    }

    /// Shows the written file in place of the old mapping. Saved edits leave
    /// the overlay; the undo history stays.
    private func adopt(_ file: MappedFile, indexer: LargeFileIndexer, at target: URL,
                       replacing old: LargeCSVSource, saved edits: [CellKey: String]) {
        for (key, text) in edits where old.overlay[key] == text {
            old.overlay.remove(key)
        }
        let source = LargeCSVSource(file: file, configuration: configuration, indexer: indexer,
                                    overlay: old.overlay, columnIDs: old.columnIDs)
        setFileURL(target)
        publishLarge(source: source, configuration: configuration, phase: phase,
                     access: SecurityScope(target))
    }
}
