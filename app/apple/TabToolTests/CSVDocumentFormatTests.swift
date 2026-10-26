import Foundation
import Testing
import TabToolKit
@testable import TabTool

/// Writes `contents` to a unique file in the temporary directory.
private func tempFile(_ contents: String) throws -> URL {
    try tempFile(Data(contents.utf8))
}

/// Writes `data` to a unique file in the temporary directory.
private func tempFile(_ data: Data) throws -> URL {
    let url = FileManager.default.temporaryDirectory
        .appendingPathComponent("TabToolTests-\(UUID().uuidString).csv")
    try data.write(to: url)
    return url
}

@Suite struct CSVDocumentFormatTests {
    @Test @MainActor func decimalChangeConvertsNumericCellsAndUndoes() async throws {
        let um = UndoManager()
        // A semicolon-separated file keeps `1,50` in one cell; the decimal
        // mark is metadata and never splits fields.
        let doc = CSVDocument(data: Data("1,50\n".utf8),
                              configuration: CSVConfiguration(columnSeparator: ";",
                                                              decimalMark: ","),
                              undoManager: um)
        await doc.load()
        doc.setDecimalMark(".")
        #expect(doc.table?.cell(row: 0, column: 0) == "1.50")
        um.undo()
        #expect(doc.table?.cell(row: 0, column: 0) == "1,50")
    }

    @Test @MainActor func newDocumentDecimalAndHeaderTransformTypedTable() async {
        let doc = CSVDocument()
        await doc.load()
        doc.setCell("1.50", row: 0, column: 0)
        doc.setDecimalMark(",")
        #expect(doc.table?.cell(row: 0, column: 0) == "1,50")
        doc.setHeaderEnabled(true)
        #expect(doc.table?.headers?.first == "1,50")
        #expect(doc.table?.rowCount == 0)
    }

    @Test @MainActor func headerToggleMovesFirstRowAndUndoes() async throws {
        let um = UndoManager()
        let doc = CSVDocument(data: Data("h1,h2\na,b\n".utf8),
                              configuration: CSVConfiguration(firstRowAsHeader: true),
                              undoManager: um)
        await doc.load()
        #expect(doc.table?.headers == ["h1", "h2"])
        doc.setHeaderEnabled(false)
        #expect(doc.table?.headers == nil)
        #expect(doc.table?.cell(row: 0, column: 0) == "h1")
        um.undo()
        #expect(doc.table?.headers == ["h1", "h2"])
    }

    @Test @MainActor func dirtyReparseRequiresConfirmation() async throws {
        let url = try tempFile("a,b\nc,d\n")
        defer { try? FileManager.default.removeItem(at: url) }
        let um = UndoManager()
        let doc = CSVDocument(data: try Data(contentsOf: url), fileURL: url,
                              configuration: .init(), undoManager: um)
        await doc.load()
        doc.setCell("x", row: 0, column: 0)
        #expect(doc.hasUnsavedChanges())
        await #expect(throws: DocumentError.requiresConfirmation) {
            try await doc.applyParsingConfiguration(
                CSVConfiguration(columnSeparator: ";"), force: false)
        }
    }

    @Test @MainActor func forcedReparseReplacesTableAndUndoes() async throws {
        let url = try tempFile("a,b\nc,d\n")
        defer { try? FileManager.default.removeItem(at: url) }
        let um = UndoManager()
        let doc = CSVDocument(data: try Data(contentsOf: url), fileURL: url,
                              configuration: .init(), undoManager: um)
        await doc.load()
        doc.setCell("x", row: 0, column: 0)
        try await doc.applyParsingConfiguration(
            CSVConfiguration(columnSeparator: ";"), force: true)
        #expect(doc.table?.columns.count == 1)
        #expect(doc.table?.cell(row: 0, column: 0) == "a,b")
        um.undo()
        #expect(doc.table?.cell(row: 0, column: 0) == "x")
        #expect(doc.configuration.columnSeparator == ",")
    }

    @Test @MainActor func failedReparseKeepsTableAndConfiguration() async throws {
        let url = try tempFile("a,\"b\n")
        defer { try? FileManager.default.removeItem(at: url) }
        let doc = CSVDocument(data: try Data(contentsOf: url), fileURL: url,
                              configuration: CSVConfiguration(quoteCharacter: ""))
        await doc.load()
        let before = doc.table
        let beforeConfig = doc.configuration
        await #expect(throws: CSVError.malformedQuotedField) {
            try await doc.applyParsingConfiguration(CSVConfiguration(), force: true)
        }
        #expect(doc.table == before)
        #expect(doc.configuration == beforeConfig)
    }

    @Test @MainActor func structuralOperationsRegisterUndo() async throws {
        let um = UndoManager()
        let doc = CSVDocument(data: Data("a,b\n".utf8), configuration: .init(),
                              undoManager: um)
        await doc.load()
        doc.addRow(at: 1)
        #expect(doc.table?.rowCount == 2)
        um.undo()
        #expect(doc.table?.rowCount == 1)
        doc.addColumn(at: 2)
        #expect(doc.table?.columns.count == 3)
        um.undo()
        #expect(doc.table?.columns.count == 2)
    }

    @Test @MainActor func pasteInsertsRowsAddsColumnsAndConvertsDecimal() async throws {
        let doc = CSVDocument(data: Data("a,b\n".utf8),
                              configuration: CSVConfiguration(decimalMark: ","))
        await doc.load()
        doc.selection = GridSelection(rows: IndexSet([0]))
        doc.pasteTSV("1.5\t2\t3", locale: Locale(identifier: "en_US"))
        #expect(doc.table?.rowCount == 2)
        #expect(doc.table?.columns.count == 3)
        #expect(doc.table?.cell(row: 1, column: 0) == "1,5")
        #expect(doc.table?.cell(row: 1, column: 2) == "3")
        #expect(doc.selection.rows == IndexSet([1]))
    }

    @Test @MainActor func copySelectionUsesVisualColumnOrder() async throws {
        let doc = CSVDocument(data: Data("a,b\nc,d\n".utf8), configuration: .init())
        await doc.load()
        doc.moveColumn(from: 1, to: 0)
        doc.selection = GridSelection(rows: IndexSet([0]))
        #expect(doc.copySelection(locale: Locale(identifier: "en_US")) == "b\ta")
    }

    @Test @MainActor func writeIssueReportsEncodingAndAmbiguity() async throws {
        let doc = CSVDocument(data: Data("x\n".utf8),
                              configuration: CSVConfiguration(encoding: .windowsCP1252))
        await doc.load()
        doc.setCell("Привет", row: 0, column: 0)
        #expect(doc.writeIssue == .unencodableText)
        #expect(doc.phase == .needsAttention(.unencodableText))
        #expect(throws: CSVError.unencodableText) { _ = try doc.encodedData() }
        doc.setCell("ok", row: 0, column: 0)
        #expect(doc.phase == .ready)

        let noQuote = CSVDocument(data: Data("x\n".utf8),
                                  configuration: CSVConfiguration(quoteCharacter: ""))
        await noQuote.load()
        noQuote.setCell("a,b", row: 0, column: 0)
        #expect(noQuote.writeIssue == .ambiguousOutput)
        #expect(noQuote.phase == .needsAttention(.ambiguousOutput))
    }

    @Test @MainActor func hasUnsavedChangesTracksDiskBytes() async throws {
        let url = try tempFile("a,b\n")
        defer { try? FileManager.default.removeItem(at: url) }
        let doc = CSVDocument(data: try Data(contentsOf: url), fileURL: url,
                              configuration: .init())
        await doc.load()
        #expect(!doc.hasUnsavedChanges())
        doc.setCell("x", row: 0, column: 0)
        #expect(doc.hasUnsavedChanges())
        try doc.encodedData().write(to: url, options: .atomic)
        #expect(!doc.hasUnsavedChanges())
        try Data("a,c\n".utf8).write(to: url, options: .atomic)
        #expect(doc.hasUnsavedChanges())
    }

    @Test @MainActor func cellUndoAndRedoRestoreValuesStepwise() async throws {
        let um = UndoManager()
        let doc = CSVDocument(data: Data("a\n".utf8), configuration: .init(),
                              undoManager: um)
        await doc.load()
        doc.setCell("b", row: 0, column: 0)
        doc.setCell("c", row: 0, column: 0)
        um.undo()
        #expect(doc.table?.cell(row: 0, column: 0) == "b")
        um.undo()
        #expect(doc.table?.cell(row: 0, column: 0) == "a")
        um.redo()
        #expect(doc.table?.cell(row: 0, column: 0) == "b")
        um.redo()
        #expect(doc.table?.cell(row: 0, column: 0) == "c")
    }

    @Test @MainActor func untitledConfigurationChangeOnlyUpdatesFormat() async throws {
        let um = UndoManager()
        let doc = CSVDocument(data: Data("a,b\n".utf8), fileURL: nil,
                              configuration: .init(), undoManager: um)
        await doc.load()
        try await doc.applyParsingConfiguration(
            CSVConfiguration(columnSeparator: ";"), force: false)
        #expect(doc.configuration.columnSeparator == ";")
        #expect(doc.table?.columns.count == 2)
        #expect(doc.table?.cell(row: 0, column: 0) == "a")
        um.undo()
        #expect(doc.configuration.columnSeparator == ",")
    }

    @Test @MainActor func missingFileReparseReportsUnreadableFile() async throws {
        let url = try tempFile("a\n")
        defer { try? FileManager.default.removeItem(at: url) }
        let doc = CSVDocument(data: try Data(contentsOf: url), fileURL: url,
                              configuration: .init())
        await doc.load()
        try FileManager.default.removeItem(at: url)
        await #expect(throws: DocumentError.cannotReadFile) {
            try await doc.applyParsingConfiguration(
                CSVConfiguration(columnSeparator: ";"), force: true)
        }
        #expect(doc.table?.cell(row: 0, column: 0) == "a")
        #expect(doc.configuration.columnSeparator == ",")
    }

    @Test @MainActor func detectFormatFromDiskReadsWithoutApplying() async throws {
        let url = try tempFile("a;b\n1,5;2,5\n")
        defer { try? FileManager.default.removeItem(at: url) }
        let doc = CSVDocument(data: try Data(contentsOf: url), fileURL: url,
                              configuration: .init())
        await doc.load()
        let detected = try await doc.detectFormatFromDisk()
        #expect(detected.columnSeparator == ";")
        #expect(detected.decimalMark == ",")
        #expect(doc.configuration.columnSeparator == ",")
    }

    @Test @MainActor func snapshotKeepsBOMStateAcrossReread() async throws {
        var input = Data([0xEF, 0xBB, 0xBF])
        input.append(Data("a\n".utf8))
        let url = try tempFile(input)
        defer { try? FileManager.default.removeItem(at: url) }
        let doc = CSVDocument(data: input, fileURL: url, configuration: .init())
        await doc.load()
        let snapshot = try doc.snapshot(contentType: .commaSeparatedText)
        #expect(snapshot.includesUTF8BOM)

        // A re-read from the same path without a BOM updates the document but
        // must not change the BOM a previously captured snapshot writes.
        try Data("a\n".utf8).write(to: url, options: .atomic)
        try await doc.applyParsingConfiguration(.init(), force: true)
        #expect(!doc.hadUTF8BOM)
        let encoded = try CSVDocumentIO.encodedData(for: snapshot)
        #expect(encoded.starts(with: [0xEF, 0xBB, 0xBF]))
    }

    @Test @MainActor func operationsUseDocumentedUndoActionNames() async throws {
        let um = UndoManager()
        let doc = CSVDocument(data: Data("a,b\nc,d\n".utf8), configuration: .init(),
                              undoManager: um)
        await doc.load()

        doc.setCell("x", row: 0, column: 0)
        #expect(um.undoActionName == "Edit Cell")
        doc.addRow(at: 0)
        #expect(um.undoActionName == "Add Row")
        doc.addColumn(at: 0)
        #expect(um.undoActionName == "Add Column")
        doc.moveRows(at: IndexSet([0]), to: 2)
        #expect(um.undoActionName == "Move Rows")
        doc.moveColumn(from: 0, to: 1)
        #expect(um.undoActionName == "Move Column")
        doc.setDecimalMark(",")
        #expect(um.undoActionName == "Change Format")
        doc.setHeaderEnabled(true)
        #expect(um.undoActionName == "Toggle Header")
        doc.selection = GridSelection(rows: IndexSet([0]))
        doc.pasteTSV("1", locale: Locale(identifier: "en_US"))
        #expect(um.undoActionName == "Paste Data")
        doc.deleteSelection()
        #expect(um.undoActionName == "Delete Rows")
        doc.selection = GridSelection(columns: IndexSet([0]))
        doc.deleteSelection()
        #expect(um.undoActionName == "Delete Columns")
    }

    @Test @MainActor func reparseRejectsEditsMadeWhileParsing() async throws {
        let url = try tempFile("a,b\n")
        defer { try? FileManager.default.removeItem(at: url) }
        let doc = CSVDocument(data: try Data(contentsOf: url), fileURL: url,
                              configuration: .init())
        await doc.load()
        let revision = doc.mutationRevision
        doc.setCell("x", row: 0, column: 0)
        #expect(doc.mutationRevision == revision + 1)

        let candidate = try CSVDocumentIO.parse(data: Data("a,b\n".utf8),
                                                configuration: .init())
        #expect(throws: DocumentError.requiresConfirmation) {
            try doc.applyReparsedCandidate(candidate, configuration: .init(),
                                           expectedRevision: revision)
        }
        #expect(doc.table?.cell(row: 0, column: 0) == "x")
        #expect(doc.configuration.columnSeparator == ",")
        #expect(doc.mutationRevision == revision + 1)
    }

    @Test @MainActor func overlappingRereadIsRejected() async throws {
        let url = try tempFile("a\n")
        defer { try? FileManager.default.removeItem(at: url) }
        let doc = CSVDocument(data: try Data(contentsOf: url), fileURL: url,
                              configuration: .init())
        await doc.load()
        #expect(doc.beginReread())
        defer { doc.endReread() }

        await #expect(throws: DocumentError.requiresConfirmation) {
            try await doc.applyParsingConfiguration(
                CSVConfiguration(columnSeparator: ";"), force: true)
        }
        #expect(doc.table?.cell(row: 0, column: 0) == "a")
        #expect(doc.configuration.columnSeparator == ",")
    }

    @Test @MainActor func asyncDirtyCheckMatchesDisk() async throws {
        let url = try tempFile("a,b\n")
        defer { try? FileManager.default.removeItem(at: url) }
        let doc = CSVDocument(data: try Data(contentsOf: url), fileURL: url,
                              configuration: .init())
        await doc.load()

        var dirty = await doc.hasUnsavedChangesAsync()
        #expect(!dirty)
        doc.setCell("x", row: 0, column: 0)
        dirty = await doc.hasUnsavedChangesAsync()
        #expect(dirty)
        try doc.encodedData().write(to: url, options: .atomic)
        dirty = await doc.hasUnsavedChangesAsync()
        #expect(!dirty)
        try Data("a,c\n".utf8).write(to: url, options: .atomic)
        dirty = await doc.hasUnsavedChangesAsync()
        #expect(dirty)
    }

    @Test @MainActor func deletingEveryColumnKeepsOneColumn() async throws {
        let um = UndoManager()
        let doc = CSVDocument(data: Data("h1,h2\na,b\n".utf8),
                              configuration: CSVConfiguration(firstRowAsHeader: true),
                              undoManager: um)
        await doc.load()
        doc.deleteColumns(at: IndexSet([0, 1]))
        #expect(doc.table?.columns.count == 1)
        #expect(doc.table?.rowCount == 1)
        #expect(doc.table?.headers == ["h1"])
        #expect(doc.table?.cell(row: 0, column: 0) == "a")
        um.undo()
        #expect(doc.table?.columns.count == 2)
        #expect(doc.table?.headers == ["h1", "h2"])
    }

    @Test @MainActor func deleteSelectionKeepsOneColumn() async throws {
        let doc = CSVDocument(data: Data("a,b\nc,d\n".utf8), configuration: .init())
        await doc.load()
        doc.selection = GridSelection(columns: IndexSet([0, 1]))
        doc.deleteSelection()
        #expect(doc.table?.columns.count == 1)
        #expect(doc.table?.rowCount == 2)
        #expect(doc.table?.cell(row: 0, column: 0) == "a")
        #expect(doc.selection == GridSelection())
    }

    @Test @MainActor func deletingTheOnlyColumnKeepsIt() async throws {
        let doc = CSVDocument(data: Data("a\nb\n".utf8), configuration: .init())
        await doc.load()
        doc.deleteColumns(at: IndexSet([0]))
        #expect(doc.table?.columns.count == 1)
        #expect(doc.table?.rowCount == 2)
        #expect(doc.table?.cell(row: 0, column: 0) == "a")
    }
}
