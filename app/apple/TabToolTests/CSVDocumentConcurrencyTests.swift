import Foundation
import Testing
import TabToolKit
import UniformTypeIdentifiers
@testable import TabTool

/// Autosave calls the save protocol from a background queue, so snapshotting
/// and serialization must never read main-actor state.
@Suite struct CSVDocumentConcurrencyTests {
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
