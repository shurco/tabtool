import Foundation
import SwiftUI
import TabToolKit
import UniformTypeIdentifiers

/// The save-time value a document publishes to background readers.
enum CSVDocumentCachedSnapshot: Sendable {
    /// A snapshot that can be serialized.
    case ready(CSVDocumentSnapshot)
    /// Saving is blocked by a validation issue.
    case needsAttention(CSVError)
    /// There is no table to save yet.
    case notReady

    /// The snapshot to save, or the typed error that blocks saving.
    func snapshot() throws -> CSVDocumentSnapshot {
        switch self {
        case .ready(let snapshot):
            return snapshot
        case .needsAttention(let error):
            throw error
        case .notReady:
            throw DocumentError.notReady
        }
    }
}

/// A lock-protected snapshot shared by the main actor and autosave.
///
/// `ReferenceFileDocument.snapshot(contentType:)` runs on a background queue,
/// so it cannot read document state. The document refreshes this cache on the
/// main actor after every save-relevant change; readers take one consistent
/// value under the lock.
final class CSVDocumentSnapshotCache: @unchecked Sendable {
    // `value` is only accessed while `lock` is held, which is the reason this
    // class is `@unchecked Sendable`.
    private let lock = NSLock()
    private var value = CSVDocumentCachedSnapshot.notReady

    func store(_ snapshot: CSVDocumentCachedSnapshot) {
        lock.lock()
        defer { lock.unlock() }
        value = snapshot
    }

    func currentSnapshot() throws -> CSVDocumentSnapshot {
        lock.lock()
        let cached = value
        lock.unlock()
        return try cached.snapshot()
    }
}

/// Snapshot and file-wrapper serialization for saving.
extension CSVDocument {
    /// Returns the cached snapshot without reading main-actor state.
    ///
    /// Autosave calls this off the main actor; it must never touch `table`,
    /// `configuration`, or `phase` directly.
    nonisolated func snapshot(contentType: UTType) throws -> CSVDocumentSnapshot {
        try snapshotCache.currentSnapshot()
    }

    /// Serializes the current content for saving and tests.
    func encodedData() throws -> Data {
        try CSVDocumentIO.encodedData(for: snapshot(contentType: .commaSeparatedText))
    }

    /// Serializes the snapshot parameter only; no document state is read.
    nonisolated func fileWrapper(
        snapshot: CSVDocumentSnapshot,
        configuration: ReferenceFileDocument.WriteConfiguration
    ) throws -> FileWrapper {
        FileWrapper(regularFileWithContents: try CSVDocumentIO.serializedData(for: snapshot))
    }

    /// Rebuilds the cache from main-actor state after a save-relevant change.
    func refreshSnapshotCache() {
        guard let table else {
            snapshotCache.store(.notReady)
            return
        }
        switch phase {
        case .ready:
            snapshotCache.store(.ready(CSVDocumentSnapshot(
                table: table,
                configuration: configuration,
                includesUTF8BOM: hadUTF8BOM
            )))
        case .needsAttention(let issue):
            snapshotCache.store(.needsAttention(CSVDocumentIO.error(for: issue)))
        case .loading, .readFailed, .unreadableFile:
            snapshotCache.store(.notReady)
        }
    }
}
