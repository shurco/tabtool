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

/// Opening and watching a memory-mapped file.
extension CSVDocument {
    /// Whether a file of this size opens through the memory-mapped path.
    nonisolated static func usesLargeMode(forFileSize size: Int64) -> Bool {
        size >= LargeFile.thresholdBytes
    }

    var hasContent: Bool { table != nil || largeSource != nil }

    /// What the grid and the status bar read.
    var rowSource: (any RowSource)? { table ?? largeSource }

    private enum OpenFailure: Error {
        case csv(CSVError)
        case unreadable
    }

    /// Maps the file, detects its format from a sample and starts indexing.
    ///
    /// `overlay` carries edits over when the file is reloaded.
    func openLargeFile(keeping overlay: CellOverlay = CellOverlay()) async {
        guard let fileURL else {
            publishLarge(source: nil, configuration: configuration, phase: .unreadableFile, access: nil)
            return
        }
        let access = SecurityScope(fileURL)
        let prefersChinese = prefersChineseEncoding
        let opened = await Task.detached(priority: .userInitiated) {
            () -> Result<(MappedFile, CSVConfiguration), OpenFailure> in
            do {
                let file = try MappedFile(url: fileURL)
                return .success((file, try LargeFileSniffer.sniff(file, prefersChineseEncoding: prefersChinese)))
            } catch let error as CSVError {
                return .failure(.csv(error))
            } catch {
                return .failure(.unreadable)
            }
        }.value
        guard !Task.isCancelled else { return }

        switch opened {
        case .success((let file, let detected)):
            let indexer = LargeFileIndexer(file: file, configuration: detected)
            indexer.start { _ in }
            let source = LargeCSVSource(file: file, configuration: detected, indexer: indexer,
                                        overlay: overlay)
            publishLarge(source: source, configuration: detected, phase: .ready, access: access)
            pollLargeIndexing(of: source)
        case .failure(.csv(let error)):
            publishLarge(source: nil, configuration: configuration, phase: .readFailed(error), access: nil)
        case .failure(.unreadable):
            publishLarge(source: nil, configuration: configuration, phase: .unreadableFile, access: nil)
        }
    }

    /// Re-opens the file, keeping the edits made so far.
    func reloadLargeFile() async {
        await openLargeFile(keeping: largeSource?.overlay ?? CellOverlay())
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
}
