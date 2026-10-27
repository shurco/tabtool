import Foundation
import TabToolKit

/// Format changes that re-read the file and the dirty checks used to confirm
/// them.
extension CSVDocument {
    /// Applies a parsing configuration from the format bar.
    ///
    /// With a file, the bytes are re-read and parsed off the main actor into a
    /// candidate; the table and configuration are replaced only when parsing
    /// succeeds and the document has not changed since the re-read started.
    /// Unsaved edits require `force` (the UI asks for confirmation first). An
    /// untitled document has no bytes to re-read, so only its configuration
    /// changes.
    func applyParsingConfiguration(_ new: CSVConfiguration,
                                   force: Bool = false) async throws {
        switch phase {
        case .loading:
            throw DocumentError.notReady
        case .ready, .needsAttention, .readFailed, .unreadableFile:
            break
        }

        // Commit the field editor before the revision is captured so a re-read
        // cannot race with text that is still in the field editor.
        endEditing()

        if isLargeMode {
            try await reopenLargeFile(with: new, force: force)
            return
        }

        guard let fileURL else {
            guard let table else { throw DocumentError.notReady }
            applyChange(table: table, configuration: new,
                        includesUTF8BOM: hadUTF8BOM, actionName: "Change Format")
            return
        }

        guard beginReread() else {
            throw DocumentError.requiresConfirmation
        }
        defer { endReread() }

        let expectedRevision = mutationRevision
        if !force, table != nil {
            let dirty = await hasUnsavedChangesAsync()
            guard !dirty, mutationRevision == expectedRevision else {
                throw DocumentError.requiresConfirmation
            }
        }

        let parsed = try await Task.detached(priority: .userInitiated) {
            let data = try CSVDocumentIO.readData(at: fileURL)
            return try CSVDocumentIO.parse(data: data, configuration: new)
        }.value

        try applyReparsedCandidate(parsed, configuration: new,
                                   expectedRevision: expectedRevision)
    }

    /// Applies a re-read candidate only when the state has not changed since
    /// `expectedRevision`.
    ///
    /// The async re-read captures the revision before its awaits. A cell edit,
    /// format change, undo, or Save As during the parse makes the candidate
    /// stale, so it must not replace the newer state. Committing the field
    /// editor first also bumps the revision when text is still being edited,
    /// which rejects the stale candidate instead of overwriting the edit.
    func applyReparsedCandidate(_ candidate: CSVDocumentIO.ParsedTable,
                                configuration newConfiguration: CSVConfiguration,
                                expectedRevision: UInt64,
                                actionName: String = "Change Format") throws {
        endEditing()
        guard mutationRevision == expectedRevision else {
            throw DocumentError.requiresConfirmation
        }
        applyChange(table: candidate.table, configuration: newConfiguration,
                    includesUTF8BOM: candidate.includesUTF8BOM,
                    actionName: actionName)
    }

    /// Whether File > Revert to Saved can run.
    var canRevertToSaved: Bool {
        fileURL != nil && hasContent && phase != .loading && !isSaving
    }

    /// Replaces the table with the file at `fileURL` parsed with the current
    /// configuration.
    ///
    /// The caller confirms a dirty document first. The replacement is one
    /// undoable action, so a mistaken revert can be undone like any other
    /// structural change.
    func revertToSaved() async throws {
        if largeSource != nil {
            revertLargeEdits()
            return
        }
        guard let fileURL, table != nil else {
            throw DocumentError.notReady
        }
        guard beginReread() else {
            throw DocumentError.requiresConfirmation
        }
        defer { endReread() }

        endEditing()
        let expectedRevision = mutationRevision
        let configuration = configuration
        let parsed = try await Task.detached(priority: .userInitiated) {
            let data = try CSVDocumentIO.readData(at: fileURL)
            return try CSVDocumentIO.parse(data: data, configuration: configuration)
        }.value

        try applyReparsedCandidate(parsed, configuration: configuration,
                                   expectedRevision: expectedRevision,
                                   actionName: "Revert to Saved")
    }

    /// Runs the format heuristic over the current bytes of the file on disk.
    ///
    /// Returns the current configuration for an untitled document. The result
    /// is not applied; the caller passes it to `applyParsingConfiguration`.
    func detectFormatFromDisk() async throws -> CSVConfiguration {
        guard let fileURL else { return configuration }
        let prefersChinese = prefersChineseEncoding
        if isLargeMode {
            return try await Task.detached(priority: .userInitiated) {
                try Self.mapLargeFile(at: fileURL, configuration: nil, prefersChineseEncoding: prefersChinese).1
            }.value
        }
        return try await Task.detached(priority: .userInitiated) {
            let data = try CSVDocumentIO.readData(at: fileURL)
            return CSVFormatDetector(data: data,
                                     prefersChineseEncoding: prefersChinese).detect()
        }.value
    }

    /// Whether the in-memory table differs from the file at `fileURL`.
    ///
    /// The file is parsed with the current configuration and compared by
    /// headers and logical cells, so formatting differences such as the
    /// writer's quoting do not count. A missing or unparseable file counts as
    /// dirty.
    ///
    /// This synchronous wrapper is kept for callers that already run on the
    /// main actor and for tests. Format re-reading uses
    /// `hasUnsavedChangesAsync()` so large files are parsed off the main actor.
    func hasUnsavedChanges() -> Bool {
        if isLargeMode { return hasUnsavedLargeEdits }
        guard let table, let fileURL else { return true }
        guard let data = try? CSVDocumentIO.readData(at: fileURL),
              let parsed = try? CSVDocumentIO.parse(data: data,
                                                    configuration: configuration) else {
            return true
        }
        return parsed.table != table
    }

    /// Async dirty check that reads and parses the file off the main actor.
    ///
    /// The comparison happens on the main actor against the current table.
    /// A mutation that lands while the file is being read returns `true`
    /// conservatively. A missing or unparseable file also counts as dirty.
    func hasUnsavedChangesAsync() async -> Bool {
        if isLargeMode { return hasUnsavedLargeEdits }
        guard table != nil, let fileURL else { return true }
        let configuration = configuration
        let revision = mutationRevision
        let parsedTable: CSVTable? = await Task.detached(priority: .userInitiated) {
            guard let data = try? CSVDocumentIO.readData(at: fileURL),
                  let parsed = try? CSVDocumentIO.parse(data: data,
                                                        configuration: configuration) else {
                return nil
            }
            return parsed.table
        }.value
        guard let parsedTable, let table, mutationRevision == revision else {
            return true
        }
        return parsedTable != table
    }
}
