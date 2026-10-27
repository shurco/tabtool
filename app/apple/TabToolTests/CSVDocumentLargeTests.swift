import AppKit
import Foundation
import Testing
import TabToolKit
@testable import TabTool

/// A temporary directory removed with the test.
final class LargeTestDirectory {
    let url = FileManager.default.temporaryDirectory
        .appendingPathComponent("tabtool-large-\(UUID().uuidString)", isDirectory: true)

    init() { try? FileManager.default.createDirectory(at: url, withIntermediateDirectories: true) }
    deinit { try? FileManager.default.removeItem(at: url) }

    func file(_ data: Data, name: String = "big.csv") throws -> URL {
        let target = url.appendingPathComponent(name)
        try data.write(to: target)
        return target
    }

    /// Rows like `r5,v5,"q 5"`; the second field repeats every seven rows.
    func csv(rows: Int) throws -> URL {
        try file(Data((0..<rows).map { "r\($0),v\($0 % 7),\"q \($0)\"" }.joined(separator: "\n").utf8))
    }
}

@MainActor
func openedLargeDocument(_ url: URL) async throws -> CSVDocument {
    let document = CSVDocument(largeFileURL: url)
    document.setFileURL(url)
    await document.load()
    try await waitForIndexing(of: document)
    return document
}

@MainActor
func waitForIndexing(of document: CSVDocument) async throws {
    var attempts = 0
    while document.phase == .ready, document.largeStatus != .ready, attempts < 200 {
        try await Task.sleep(for: .milliseconds(25))
        attempts += 1
    }
}

@Suite @MainActor struct CSVDocumentLargeTests {
    @Test func opensLargeAndPublishesSource() async throws {
        let dir = LargeTestDirectory()
        let document = try await openedLargeDocument(dir.csv(rows: 5000))
        #expect(document.table == nil)
        #expect(document.isLargeMode)
        #expect(document.hasContent)
        #expect(document.phase == .ready)
        #expect(document.largeStatus == .ready)
        let headerRows = document.configuration.firstRowAsHeader ? 1 : 0
        #expect(document.largeSource?.rowCount == 5000 - headerRows)
        #expect(document.rowSource?.cell(row: 4999 - headerRows, column: 0) == "r4999")
    }

    @Test func routesBySizeAgainstTheThreshold() {
        #expect(!CSVDocument.usesLargeMode(forFileSize: LargeFile.thresholdBytes - 1))
        #expect(CSVDocument.usesLargeMode(forFileSize: LargeFile.thresholdBytes))
    }

    @Test func smallDocumentStaysInMemory() async {
        let document = CSVDocument(data: Data("a,b\n1,2\n".utf8))
        await document.load()
        #expect(!document.isLargeMode)
        #expect(document.table != nil)
        #expect(document.largeSource == nil)
        #expect(document.largeStatus == .none)
    }

    @Test func structuralCommandsAreDisabled() async throws {
        let dir = LargeTestDirectory()
        let document = try await openedLargeDocument(dir.csv(rows: 300))
        document.selection = GridSelection(rows: IndexSet(integer: 1), columns: IndexSet(integer: 1))
        #expect(!document.canDeleteSelection)
        #expect(!document.isEditableContent)
        #expect(!document.canSortSelection)
        #expect(!document.canMoveSelectedRows(by: 1))
        #expect(!document.canRenameSelectedColumn)
    }

    @Test func utf16FileGivesReadFailed() async throws {
        let dir = LargeTestDirectory()
        let url = try dir.file(Data("a,b\n1,2\n".data(using: .utf16)!))
        let document = try await openedLargeDocument(url)
        #expect(document.phase == .readFailed(.unsupportedLargeFileEncoding))
        #expect(document.largeSource == nil)
    }

    @Test func missingFileIsUnreadable() async throws {
        let document = CSVDocument(largeFileURL: nil)
        document.setFileURL(FileManager.default.temporaryDirectory.appendingPathComponent("missing-\(UUID()).csv"))
        await document.load()
        #expect(document.phase == .unreadableFile)
    }

    @Test func gridShowsIndexedRowsBeforeIndexingFinishes() async throws {
        let dir = LargeTestDirectory()
        let file = try MappedFile(url: dir.csv(rows: 20_000))
        let indexer = LargeFileIndexer(file: file, configuration: CSVConfiguration(), stepBytes: 4096,
                                       progressInterval: 0)
        indexer.start { _ in indexer.cancel() }
        indexer.wait()
        let source = LargeCSVSource(file: file, configuration: CSVConfiguration(), indexer: indexer)
        #expect(!indexer.isFinished)
        #expect(source.rowCount > 0 && source.rowCount < 20_000)

        let document = await GridTestSupport.makeLoadedNewDocument()
        let (coordinator, tableView, _) = GridTestSupport.makeGrid(document)
        withExtendedLifetime(coordinator) {
            coordinator.update(presenter: GridPresenter(table: source, configuration: CSVConfiguration()),
                               isEditable: false)
            #expect(tableView.numberOfRows == source.rowCount)
            #expect(tableView.numberOfColumns == 3)
            #expect(!tableView.allowsColumnReordering)
        }
    }

    @Test func statusTextShowsIndexingProgress() {
        let text = LargeStatusText.indexing(fraction: 0.376, rows: 12_345_678)
        #expect(text.hasPrefix("Indexing… 37%"))
        #expect(text.contains("rows so far"))
    }
}
