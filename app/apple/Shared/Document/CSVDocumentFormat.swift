import Foundation
import TabToolKit

/// Format changes that re-read the file and the dirty check used to confirm
/// them.
extension CSVDocument {
    /// Applies a parsing configuration from the format bar.
    ///
    /// With a file, the bytes are re-read and parsed off the main actor into a
    /// candidate; the table and configuration are replaced only when parsing
    /// succeeds. Unsaved edits require `force` (the UI asks for confirmation
    /// first). An untitled document has no bytes to re-read, so only its
    /// configuration changes.
    func applyParsingConfiguration(_ new: CSVConfiguration,
                                   force: Bool = false) async throws {
        switch phase {
        case .loading:
            throw DocumentError.notReady
        case .ready, .needsAttention, .readFailed, .unreadableFile:
            break
        }

        guard let fileURL else {
            guard let table else { throw DocumentError.notReady }
            applyChange(table: table, configuration: new,
                        includesUTF8BOM: hadUTF8BOM, actionName: "Change Format")
            return
        }

        if !force, table != nil, hasUnsavedChanges() {
            throw DocumentError.requiresConfirmation
        }

        let parsed = try await Task.detached(priority: .userInitiated) {
            let data = try CSVDocumentIO.readData(at: fileURL)
            return try CSVDocumentIO.parse(data: data, configuration: new)
        }.value

        applyChange(table: parsed.table, configuration: new,
                    includesUTF8BOM: parsed.includesUTF8BOM,
                    actionName: "Change Format")
    }

    /// Runs the format heuristic over the current bytes of the file on disk.
    ///
    /// Returns the current configuration for an untitled document. The result
    /// is not applied; the caller passes it to `applyParsingConfiguration`.
    func detectFormatFromDisk() async throws -> CSVConfiguration {
        guard let fileURL else { return configuration }
        let prefersChinese = prefersChineseEncoding
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
    func hasUnsavedChanges() -> Bool {
        guard let table, let fileURL else { return true }
        guard let data = try? CSVDocumentIO.readData(at: fileURL),
              let parsed = try? CSVDocumentIO.parse(data: data,
                                                    configuration: configuration) else {
            return true
        }
        return parsed.table != table
    }
}
