import Foundation
import Testing
import TabToolKit
@testable import TabTool

@Suite @MainActor struct CSVDocumentLargeSavingTests {
    /// A ready large document over rows like `r5,v5,"q 5"`, without a header row.
    private func open(_ dir: LargeTestDirectory, rows: Int = 300) async throws -> (CSVDocument, URL) {
        let url = try dir.csv(rows: rows)
        let document = try await openedLargeDocument(url)
        if document.configuration.firstRowAsHeader { document.setHeaderEnabled(false) }
        return (document, url)
    }

    private func lines(_ url: URL) throws -> [String] {
        try String(contentsOf: url, encoding: .utf8).components(separatedBy: "\n")
    }

    private func inode(_ url: URL) throws -> Int? {
        try FileManager.default.attributesOfItem(atPath: url.path)[.systemFileNumber] as? Int
    }

    @Test func saveWritesTheEditsAndAdoptsTheWrittenFile() async throws {
        let dir = LargeTestDirectory()
        let (document, url) = try await open(dir)
        let original = try lines(url)
        let before = try #require(document.largeSource)
        document.setCell("X", row: 2, column: 1)

        try await document.saveLargeFile()

        let saved = try lines(url)
        #expect(saved.count == original.count)
        #expect(saved[2] == "\"r2\",\"X\",\"q 2\"")
        #expect(saved.indices.allSatisfy { $0 == 2 || saved[$0] == original[$0] })
        let after = try #require(document.largeSource)
        #expect(after !== before)
        #expect(after.columnIDs == before.columnIDs)
        #expect(after.overlay.isEmpty)
        #expect(after.cell(row: 2, column: 1) == "X")
        #expect(after.staleness == .unchanged)
        #expect(document.largeStatus == .ready)
        #expect(!document.hasUnsavedLargeEdits)
    }

    @Test func saveWithoutEditsLeavesTheFileAlone() async throws {
        let dir = LargeTestDirectory()
        let (document, url) = try await open(dir)
        let before = try inode(url)
        try await document.saveLargeFile()
        #expect(try inode(url) == before)
    }

    @Test func saveIsRefusedUntilIndexingFinishes() async throws {
        let dir = LargeTestDirectory()
        let (document, _) = try await open(dir)
        document.setCell("X", row: 2, column: 1)
        document.setLargeStatus(.indexing(0.5))
        await #expect(throws: LargeFileSaveError.indexingNotFinished) {
            try await document.saveLargeFile()
        }
        #expect(document.hasUnsavedLargeEdits)
    }

    @Test func editsAreRefusedWhileSaving() async throws {
        let dir = LargeTestDirectory()
        let (document, _) = try await open(dir)
        document.setLargeStatus(.saving(0.3))
        document.setCell("X", row: 2, column: 1)
        #expect(document.notice == .editRejected(.rowNotEditable))
        #expect(!document.hasUnsavedLargeEdits)
        #expect(!document.canRevertToSaved)
    }

    @Test func undoAfterSaveRestoresThePreEditValue() async throws {
        let dir = LargeTestDirectory()
        let (document, _) = try await open(dir)
        document.setCell("X", row: 2, column: 1)
        try await document.saveLargeFile()

        document.undoManager?.undo()
        let source = try #require(document.largeSource)
        #expect(source.cell(row: 2, column: 1) == "v2")
        #expect(document.hasUnsavedLargeEdits)
        document.undoManager?.redo()
        #expect(source.cell(row: 2, column: 1) == "X")
        #expect(!document.hasUnsavedLargeEdits)
    }

    @Test func saveAsWritesTheNewFileAndKeepsTheOriginal() async throws {
        let dir = LargeTestDirectory()
        let (document, url) = try await open(dir)
        let original = try Data(contentsOf: url)
        let copy = dir.url.appendingPathComponent("copy.csv")
        document.setCell("X", row: 2, column: 1)

        try await document.saveLargeFile(to: copy)

        #expect(try Data(contentsOf: url) == original)
        #expect(try lines(copy)[2] == "\"r2\",\"X\",\"q 2\"")
        #expect(document.fileURL == copy)
        #expect(document.largeSource?.staleness == .unchanged)
        #expect(!document.hasUnsavedLargeEdits)
    }

    @Test func truncatedFileIsMarkedStale() async throws {
        let dir = LargeTestDirectory()
        let (document, url) = try await open(dir)
        try Data("r0".utf8).write(to: url)
        document.refreshLargeStaleness()
        #expect(document.largeStatus == .stale(appended: false))
        await #expect(throws: LargeFileSaveError.sourceChanged) {
            try await document.saveLargeFile(to: dir.url.appendingPathComponent("copy.csv"))
        }
    }

    @Test func appendedFileIsMarkedAppended() async throws {
        let dir = LargeTestDirectory()
        let (document, url) = try await open(dir)
        let handle = try FileHandle(forWritingTo: url)
        try handle.seekToEnd()
        try handle.write(contentsOf: Data("\nr300,v6,\"q 300\"".utf8))
        try handle.close()
        document.refreshLargeStaleness()
        #expect(document.largeStatus == .stale(appended: true))
    }

    @Test func statusTextShowsSavingProgress() {
        #expect(LargeStatusText.saving(fraction: 0.625) == "Saving… 62%")
    }

    @Test func saveErrorsHaveTheirOwnMessages() {
        #expect(FormatErrorText.alertMessage(for: LargeFileSaveError.insufficientDiskSpace)
            .hasPrefix("There is not enough free disk space to save this file."))
        #expect(FormatErrorText.alertMessage(for: LargeFileSaveError.indexingNotFinished)
            .hasPrefix("Indexing is not finished."))
        #expect(FormatErrorText.alertMessage(for: LargeFileSaveError.sourceChanged)
            .hasPrefix("The file changed on disk"))
    }
}
