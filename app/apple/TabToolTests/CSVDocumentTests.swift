import Foundation
import Testing
import TabToolKit
import UniformTypeIdentifiers
@testable import TabTool

@Suite struct CSVDocumentTests {
    @Test @MainActor func loadParsesAFileIntoTable() async throws {
        let data = Data("a,b,c\n1,2,3\n\"1,1\",\"1,2\",\"1,3\"\n".utf8)
        let doc = CSVDocument(data: data, fileURL: nil,
                              configuration: CSVConfiguration(decimalMark: ","),
                              undoManager: UndoManager())
        await doc.load()
        #expect(doc.phase == .ready)
        #expect(doc.table?.rowCount == 3)
        #expect(doc.table?.columns.count == 3)
    }

    @Test @MainActor func newDocumentIsThreeByOneAndReady() {
        let doc = CSVDocument()
        #expect(doc.phase == .ready)
        #expect(doc.table?.rowCount == 1)
        #expect(doc.table?.columns.count == 3)
        #expect(doc.table?.headers == nil)
    }

    @Test @MainActor func loadDetectsFormatWhenConfigurationIsNil() async throws {
        let doc = CSVDocument(data: Data("a;b\n1,5;2,5\n".utf8), fileURL: nil)
        await doc.load()
        #expect(doc.configuration.columnSeparator == ";")
        #expect(doc.configuration.decimalMark == ",")
    }

    @Test @MainActor func snapshotAndEncodedDataRoundTrip() async throws {
        let data = Data("a,b,c\n1,2,3\n\"1,1\",\"1,2\",\"1,3\"\n".utf8)
        let doc = CSVDocument(data: data, fileURL: nil,
                              configuration: CSVConfiguration(decimalMark: ","))
        await doc.load()
        let snap = try doc.snapshot(contentType: .commaSeparatedText)
        let encoded = try doc.encodedData()
        let expected = try CSVWriter(table: snap.table, header: snap.table.headers,
                                     configuration: snap.configuration).data()
        #expect(encoded == expected)
    }

    @Test @MainActor func snapshotBeforeLoadThrowsNotReady() {
        let doc = CSVDocument(data: Data(), fileURL: nil)
        #expect(throws: DocumentError.notReady) {
            _ = try doc.snapshot(contentType: .commaSeparatedText)
        }
    }

    @Test @MainActor func invalidDataProducesReadFailedPhase() async {
        let doc = CSVDocument(data: Data([0xFF, 0xFE, 0xFD]), fileURL: nil,
                              configuration: CSVConfiguration(encoding: .utf8))
        await doc.load()
        #expect(doc.phase == .readFailed(.cannotDecodeData))
        #expect(throws: DocumentError.notReady) { _ = try doc.encodedData() }
    }

    @Test @MainActor func ambiguousLoadedContentBlocksSaving() async {
        let doc = CSVDocument(data: Data("a\n\n".utf8), fileURL: nil,
                              configuration: CSVConfiguration(quoteCharacter: ""))
        await doc.load()
        #expect(doc.phase == .needsAttention(.ambiguousOutput))
        #expect(doc.writeIssue == .ambiguousOutput)
        #expect(throws: CSVError.ambiguousOutput) {
            _ = try doc.snapshot(contentType: .commaSeparatedText)
        }
        #expect(throws: CSVError.ambiguousOutput) {
            _ = try doc.encodedData()
        }
    }

    @Test @MainActor func retryingAFailedLoadReadsFromFileURL() async throws {
        let url = FileManager.default.temporaryDirectory
            .appendingPathComponent(UUID().uuidString)
        try Data("a,b\nc,d\n".utf8).write(to: url)
        defer { try? FileManager.default.removeItem(at: url) }

        let doc = CSVDocument(data: Data([0xFF, 0xFE, 0xFD]), fileURL: url,
                              configuration: CSVConfiguration(encoding: .utf8))
        await doc.load()
        #expect(doc.phase == .readFailed(.cannotDecodeData))

        // The initial bytes are used only for the first attempt; a retry
        // reads the current file from disk.
        await doc.load()
        #expect(doc.phase == .ready)
        #expect(doc.table?.rowCount == 2)
        #expect(doc.table?.cell(row: 0, column: 0) == "a")
    }

    @Test @MainActor func bomIsPreservedOnSave() async throws {
        var input = Data([0xEF, 0xBB, 0xBF])
        input.append(Data("a,b".utf8))
        let doc = CSVDocument(data: input, fileURL: nil,
                              configuration: CSVConfiguration())
        await doc.load()
        #expect(try doc.encodedData().starts(with: [0xEF, 0xBB, 0xBF]))
    }
}
