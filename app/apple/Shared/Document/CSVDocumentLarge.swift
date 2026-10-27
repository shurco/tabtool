import Foundation
import TabToolKit

/// Where a large file stands: being indexed, ready, or no longer matching disk.
enum LargeStatus: Equatable {
    /// Not a large file.
    case none
    /// The row index is being built; the value is the share of the file scanned.
    case indexing(Double)
    case ready
    /// The file changed on disk. `appended` is true when it only grew.
    case stale(appended: Bool)
}

/// Something the window tells the user once, then forgets.
enum DocumentNotice: Equatable {
    /// A large-file cell edit was refused; nothing changed.
    case editRejected(CSVError)
    /// A copy would produce more than `LargeFile.maxCopyCells` cells.
    case copyLimitExceeded
}

/// Opening and watching a memory-mapped file.
extension CSVDocument {
    /// Whether a file of this size opens through the memory-mapped path.
    nonisolated static func usesLargeMode(forFileSize size: Int64) -> Bool {
        size >= LargeFile.thresholdBytes
    }

    /// Whether the index of a large file is still being built.
    var isIndexing: Bool {
        if case .indexing = largeStatus { return true }
        return false
    }

    var hasContent: Bool { table != nil || largeSource != nil }

    /// What the grid and the status bar read.
    var rowSource: (any RowSource)? { table ?? largeSource }

    /// Whether cells can be edited now: always in memory, and in large mode
    /// once the index is complete.
    var canEditCells: Bool { !isLargeMode || largeStatus == .ready }

    /// Maps `url`, then detects the format from a sample (`configuration` nil)
    /// or checks the given one.
    nonisolated static func mapLargeFile(at url: URL, configuration: CSVConfiguration?,
                                         prefersChineseEncoding: Bool) throws
        -> (MappedFile, CSVConfiguration) {
        let file: MappedFile
        do { file = try MappedFile(url: url) } catch { throw DocumentError.cannotReadFile }
        if let configuration {
            try LargeFileSniffer.check(file, configuration: configuration)
            return (file, configuration)
        }
        return (file, try LargeFileSniffer.sniff(file, prefersChineseEncoding: prefersChineseEncoding))
    }

    /// Maps the file, detects its format from a sample and starts indexing.
    func openLargeFile() async {
        guard let fileURL else {
            publishLarge(source: nil, configuration: configuration, phase: .unreadableFile, access: nil)
            return
        }
        let access = SecurityScope(fileURL)
        let prefersChinese = prefersChineseEncoding
        let opened = await Task.detached(priority: .userInitiated) {
            Result { try Self.mapLargeFile(at: fileURL, configuration: nil, prefersChineseEncoding: prefersChinese) }
        }.value
        guard !Task.isCancelled else { return }

        switch opened {
        case .success((let file, let detected)):
            startLargeSource(file: file, configuration: detected, overlay: CellOverlay(), access: access)
        case .failure(let error as CSVError):
            publishLarge(source: nil, configuration: configuration, phase: .readFailed(error), access: nil)
        case .failure:
            publishLarge(source: nil, configuration: configuration, phase: .unreadableFile, access: nil)
        }
    }

    /// Re-opens the file with the same format, keeping the edits made so far.
    func reloadLargeFile() async {
        guard let fileURL, let source = largeSource else { return }
        let access = SecurityScope(fileURL)
        let configuration = self.configuration
        guard let file = try? MappedFile(url: fileURL) else { return }
        startLargeSource(file: file, configuration: configuration, overlay: source.overlay, access: access)
    }

    /// Re-reads the file with a new format. Edits and undo history are dropped,
    /// so a dirty document needs `force`.
    func reopenLargeFile(with new: CSVConfiguration, force: Bool) async throws {
        guard let fileURL else { throw DocumentError.notReady }
        if !force, largeSource?.overlay.isEmpty == false { throw DocumentError.requiresConfirmation }
        guard beginReread() else { throw DocumentError.requiresConfirmation }
        defer { endReread() }

        let revision = mutationRevision
        let prefersChinese = prefersChineseEncoding
        let (file, configuration) = try await Task.detached(priority: .userInitiated) {
            try Self.mapLargeFile(at: fileURL, configuration: new, prefersChineseEncoding: prefersChinese)
        }.value
        guard mutationRevision == revision else { throw DocumentError.requiresConfirmation }
        undoManager?.removeAllActions()
        startLargeSource(file: file, configuration: configuration, overlay: CellOverlay(),
                         access: SecurityScope(fileURL))
    }

    private func startLargeSource(file: MappedFile, configuration: CSVConfiguration,
                                  overlay: CellOverlay, access: SecurityScope) {
        let indexer = LargeFileIndexer(file: file, configuration: configuration)
        indexer.start { _ in }
        let source = LargeCSVSource(file: file, configuration: configuration, indexer: indexer,
                                    overlay: overlay)
        publishLarge(source: source, configuration: configuration, phase: .ready, access: access)
        pollLargeIndexing(of: source)
    }

    /// Publishes the indexing progress at 4 Hz until the index is complete.
    private func pollLargeIndexing(of source: LargeCSVSource) {
        Task { [weak self] in
            while let self, self.largeSource === source {
                let finished = source.isIndexed
                self.setLargeStatus(finished ? .ready : .indexing(source.indexingFraction))
                if finished { return }
                try? await Task.sleep(for: .milliseconds(250))
            }
        }
    }

    /// Marks the document stale when the file changed on disk since it was mapped.
    func refreshLargeStaleness() {
        guard let source = largeSource, largeStatus == .ready else { return }
        switch source.staleness {
        case .unchanged: break
        case .appended: setLargeStatus(.stale(appended: true))
        case .replacedOrTruncated: setLargeStatus(.stale(appended: false))
        }
    }

    // MARK: Editing

    /// Sets one cell in the overlay and records how to undo it.
    ///
    /// A refused edit (indexing, a changed file, unreadable bytes, text the
    /// format cannot hold) leaves everything as it was and raises a notice.
    func applyLargeCellChange(_ text: String, in source: LargeCSVSource, row: Int, column: Int,
                              actionName: String) {
        let key = CellKey(row: source.physicalRow(forDisplay: row), column: column)
        let previous: String?
        do {
            previous = try source.setCell(text, row: row, column: column)
        } catch {
            notice = .editRejected(error as? CSVError ?? .rowNotEditable)
            objectWillChange.send()
            return
        }
        if previous == source.overlay[key] { return }
        didChangeLargeContent()
        registerAction(actionName) { document in
            document.restoreLargeCell(previous, at: key, actionName: actionName)
        }
    }

    /// Puts an overlay entry back (`nil` removes it) and registers the inverse.
    private func restoreLargeCell(_ text: String?, at key: CellKey, actionName: String) {
        guard let source = largeSource else { return }
        let current = source.overlay[key]
        if let text { source.overlay.set(text, for: key) } else { source.overlay.remove(key) }
        didChangeLargeContent()
        registerAction(actionName) { document in
            document.restoreLargeCell(current, at: key, actionName: actionName)
        }
    }

    /// Turns the first row into a header, or back into data. Undoable; the
    /// file and the edits are untouched.
    func setLargeHeaderEnabled(_ enabled: Bool) {
        guard let source = largeSource, largeStatus == .ready,
              configuration.firstRowAsHeader != enabled else { return }
        endEditing()
        let previous = configuration.firstRowAsHeader
        var updated = configuration
        updated.firstRowAsHeader = enabled
        source.setHeaderEnabled(enabled)
        publishLarge(source: source, configuration: updated, phase: phase, access: largeAccess)
        registerAction("Toggle Header") { document in
            document.setLargeHeaderEnabled(previous)
        }
    }

    /// Drops every edit and the undo history. The file is not read again.
    func revertLargeEdits() {
        endEditing()
        largeSource?.overlay.removeAll()
        undoManager?.removeAllActions()
        didChangeLargeContent()
    }

    /// Whether the overlay holds edits that are not saved.
    var hasUnsavedLargeEdits: Bool { largeSource?.overlay.isEmpty == false }

    private func didChangeLargeContent() {
        noteContentChanged()
        objectWillChange.send()
    }
}
