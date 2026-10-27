import Foundation
import Testing
import TabToolKit
import UniformTypeIdentifiers
@testable import TabTool

/// Autosave calls the save protocol from a background queue, so snapshotting
/// and serialization must never read main-actor state. Document creation can
/// also run off the main actor while opening a file, so initializers store
/// plain values and the main-actor `load()` publishes the visible state.
@Suite struct CSVDocumentConcurrencyTests {
    @Test @MainActor func newDocumentIsConstructedOffMainActor() async {
        let document = await Task.detached { CSVDocument() }.value

        #expect(document.phase == .loading)
        #expect(document.table == nil)

        await document.load()
        #expect(document.phase == .ready)
        #expect(document.table?.rowCount == 1)
        #expect(document.table?.columns.count == 3)
        #expect(document.table?.headers == nil)
    }

    @Test @MainActor func openedDocumentIsConstructedOffMainActor() async {
        let configuration = CSVConfiguration(decimalMark: ",")
        let data = Data("a,b\n1,2\n".utf8)

        let document = await Task.detached {
            CSVDocument(data: data, fileURL: nil, configuration: configuration)
        }.value

        #expect(document.phase == .loading)
        #expect(document.table == nil)

        await document.load()
        #expect(document.phase == .ready)
        #expect(document.configuration == configuration)
        #expect(document.table?.columns.count == 2)
        #expect(document.table?.cell(row: 1, column: 1) == "2")
    }

    @Test @MainActor func referenceFileDocumentInitializerPathRunsOffMainActor() async {
        // `FileDocumentReadConfiguration` has no public initializer, so this
        // exercises the nonisolated designated init that the document's
        // `init(configuration:)` delegates to.
        let document = await Task.detached {
            CSVDocument(data: Data("a\n1\n".utf8), fileURL: nil,
                        configuration: nil, prefersChineseEncoding: true)
        }.value

        #expect(document.prefersChineseEncoding)
        #expect(document.phase == .loading)
        #expect(document.table == nil)

        await document.load()
        #expect(document.phase == .ready)
        #expect(document.table?.columns.count == 1)
    }

    @Test @MainActor func cancellingBeforeLoadKeepsSuppliedBytes() async {
        let document = CSVDocument(data: Data("a,b\n1,2\n".utf8))

        // The view may disappear before `load()` starts. The supplied bytes
        // are the only copy at that point, so cancelling must be a no-op.
        document.cancelLoading()
        await document.load()

        #expect(document.phase == .ready)
        #expect(document.table?.cell(row: 0, column: 1) == "2")
    }

    @Test(.timeLimit(.minutes(1))) @MainActor
    func cancellingDuringLoadKeepsBytesAndReloads() async {
        let data = Data("a,b\n1,2\n".utf8)
        let starts = AsyncStream<Int>.makeStream()
        let releaseFirstLoad = DispatchSemaphore(value: 0)
        let calls = LoaderCallCounter()
        let loader: CSVDocument.Loader = { data, fileURL, configuration, prefersChinese in
            let call = calls.next()
            starts.continuation.yield(call)
            if call == 1 {
                releaseFirstLoad.wait()
            }
            return CSVDocumentIO.load(data: data, fileURL: fileURL,
                                      configuration: configuration,
                                      prefersChineseEncoding: prefersChinese)
        }
        let document = CSVDocument(data: data, loader: loader)

        let firstLoad = Task { await document.load() }
        var startIterator = starts.stream.makeAsyncIterator()
        let firstStart = await startIterator.next()
        #expect(firstStart == 1)

        // The window closes while the first parse is still in flight.
        document.cancelLoading()
        #expect(document.phase == .loading)
        #expect(document.table == nil)

        // A new load must start a fresh attempt instead of awaiting the
        // cancelled one, and the supplied bytes must still be available.
        await document.load()
        #expect(document.phase == .ready)
        #expect(document.table?.cell(row: 0, column: 1) == "2")

        // Let the cancelled parse finish; it must not publish over the
        // newer attempt or discard its result.
        releaseFirstLoad.signal()
        await firstLoad.value
        #expect(calls.count == 2)
        #expect(document.phase == .ready)
        #expect(document.table?.cell(row: 0, column: 1) == "2")
    }

    @Test @MainActor func failedDataOnlyLoadKeepsSuppliedBytes() async {
        let invalid = Data([0xFF, 0xFE, 0xFD])
        let document = CSVDocument(data: invalid,
                                   configuration: CSVConfiguration(encoding: .utf8))

        await document.load()
        #expect(document.phase == .readFailed(.cannotDecodeData))

        // A data-only document has no file to re-read. Its supplied bytes
        // must survive the failed attempt so a later load repeats the parse
        // instead of reporting an unreadable file.
        await document.load()
        #expect(document.phase == .readFailed(.cannotDecodeData))
    }

    @Test @MainActor func loadingANewDocumentIsIdempotent() async {
        let document = CSVDocument()
        await document.load()
        document.setCell("edited", row: 0, column: 0)

        await document.load()

        #expect(document.table?.columns.count == 3)
        #expect(document.table?.cell(row: 0, column: 0) == "edited")
    }

    @Test @MainActor func snapshotOffMainActorReturnsLoadedContent() async throws {
        var input = Data([0xEF, 0xBB, 0xBF])
        input.append(Data("a,b\n1,2\n".utf8))
        let configuration = CSVConfiguration(decimalMark: ",")
        let document = CSVDocument(data: input, fileURL: nil,
                                   configuration: configuration)
        await document.load()

        let snapshot = try await Task.detached {
            try document.snapshot(contentType: .commaSeparatedText)
        }.value

        #expect(snapshot.table == document.table)
        #expect(snapshot.configuration == configuration)
        #expect(snapshot.includesUTF8BOM)
        #expect(snapshot.table.cell(row: 1, column: 1) == "2")
    }

    @Test @MainActor func snapshotOffMainActorSeesCellChange() async throws {
        let document = CSVDocument(data: Data("a,b\n1,2\n".utf8), fileURL: nil,
                                   configuration: CSVConfiguration())
        await document.load()
        document.setCell("42", row: 1, column: 1)

        let snapshot = try await Task.detached {
            try document.snapshot(contentType: .commaSeparatedText)
        }.value

        #expect(snapshot.table.cell(row: 1, column: 1) == "42")
    }

    @Test @MainActor func snapshotOffMainActorFollowsUndo() async throws {
        let undoManager = UndoManager()
        let document = CSVDocument(data: Data("a,b\n1,2\n".utf8), fileURL: nil,
                                   configuration: CSVConfiguration(),
                                   undoManager: undoManager)
        await document.load()
        document.setCell("42", row: 1, column: 1)
        undoManager.undo()

        let snapshot = try await Task.detached {
            try document.snapshot(contentType: .commaSeparatedText)
        }.value

        #expect(snapshot.table.cell(row: 1, column: 1) == "2")
    }

    @Test @MainActor func snapshotOffMainActorThrowsValidationError() async {
        let document = CSVDocument(data: Data("a\n\n".utf8), fileURL: nil,
                                   configuration: CSVConfiguration(quoteCharacter: ""))
        await document.load()
        #expect(document.phase == .needsAttention(.ambiguousOutput))

        await #expect(throws: CSVError.ambiguousOutput) {
            _ = try await Task.detached {
                try document.snapshot(contentType: .commaSeparatedText)
            }.value
        }
    }

    @Test @MainActor func snapshotOffMainActorThrowsNotReadyBeforeLoad() async {
        let document = CSVDocument(data: Data("a,b\n1,2\n".utf8), fileURL: nil)

        await #expect(throws: DocumentError.notReady) {
            _ = try await Task.detached {
                try document.snapshot(contentType: .commaSeparatedText)
            }.value
        }
    }

    @Test @MainActor func snapshotOffMainActorThrowsNotReadyAfterReadFailure() async {
        let document = CSVDocument(data: Data([0xFF, 0xFE, 0xFD]), fileURL: nil,
                                   configuration: CSVConfiguration(encoding: .utf8))
        await document.load()
        #expect(document.phase == .readFailed(.cannotDecodeData))

        await #expect(throws: DocumentError.notReady) {
            _ = try await Task.detached {
                try document.snapshot(contentType: .commaSeparatedText)
            }.value
        }
    }

    @Test @MainActor func serializationOffMainActorMatchesEncodedData() async throws {
        var input = Data([0xEF, 0xBB, 0xBF])
        input.append(Data("a,b\n1,2\n".utf8))
        let document = CSVDocument(data: input, fileURL: nil,
                                   configuration: CSVConfiguration())
        await document.load()
        let snapshot = try document.snapshot(contentType: .commaSeparatedText)

        let data = try await Task.detached {
            try CSVDocumentIO.serializedData(for: snapshot)
        }.value

        #expect(data == (try document.encodedData()))
        #expect(data.starts(with: [0xEF, 0xBB, 0xBF]))
    }

    @Test func serializationOffMainActorRejectsBlockedSnapshot() async throws {
        let table = CSVTable(rows: [["a"], [""]], headers: nil)
        let snapshot = CSVDocumentSnapshot(
            table: table,
            configuration: CSVConfiguration(quoteCharacter: "")
        )

        await #expect(throws: CSVError.ambiguousOutput) {
            _ = try await Task.detached {
                try CSVDocumentIO.serializedData(for: snapshot)
            }.value
        }
    }

    @Test @MainActor func encodedDataThrowsWhileNeedsAttention() async {
        let document = CSVDocument(data: Data("a\n\n".utf8), fileURL: nil,
                                   configuration: CSVConfiguration(quoteCharacter: ""))
        await document.load()
        #expect(document.phase == .needsAttention(.ambiguousOutput))

        #expect(throws: CSVError.ambiguousOutput) {
            _ = try document.encodedData()
        }
    }
}

/// Counts loader invocations from the detached parse tasks.
private final class LoaderCallCounter: @unchecked Sendable {
    private let lock = NSLock()
    private var value = 0

    func next() -> Int {
        lock.lock()
        defer { lock.unlock() }
        value += 1
        return value
    }

    var count: Int {
        lock.lock()
        defer { lock.unlock() }
        return value
    }
}
