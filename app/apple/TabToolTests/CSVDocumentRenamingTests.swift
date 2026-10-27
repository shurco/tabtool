import Foundation
import Testing
import TabToolKit
@testable import TabTool

@Suite @MainActor struct CSVDocumentRenamingTests {
    private final class RecordingSession: DocumentEditingSession {
        var renamedColumns: [Int] = []

        func endEditing() {}

        func beginRenamingColumn(at column: Int) {
            renamedColumns.append(column)
        }
    }

    private func loaded(_ csv: String, header: Bool = true)
        async -> (CSVDocument, UndoManager) {
        let undoManager = UndoManager()
        let doc = CSVDocument(data: Data(csv.utf8),
                              configuration: CSVConfiguration(firstRowAsHeader: header),
                              undoManager: undoManager)
        await doc.load()
        return (doc, undoManager)
    }

    @Test func renameIsOneNamedUndoableStep() async {
        let (doc, um) = await loaded("a,b\n1,2\n")
        doc.renameColumn(at: 1, to: "x")
        #expect(doc.table?.headers == ["a", "x"])
        #expect(um.undoActionName == "Rename Column")

        um.undo()
        #expect(doc.table?.headers == ["a", "b"])
        um.redo()
        #expect(doc.table?.headers == ["a", "x"])
    }

    @Test func renameKeepsTheSortAndColumnIdentity() async {
        let (doc, um) = await loaded("k,v\nb,1\na,2\n")
        doc.sortRows(column: 0, direction: .ascending)
        let sort = doc.activeSort
        let ids = doc.table?.columns.map(\.id)

        doc.renameColumn(at: 0, to: "key")
        #expect(doc.activeSort == sort)
        #expect(doc.table?.columns.map(\.id) == ids)
        #expect(doc.table?.columns[0].cells == ["a", "b"])

        um.undo()
        #expect(doc.activeSort == sort)
    }

    @Test func renameToTheSameNameRecordsNothing() async {
        let (doc, um) = await loaded("a,b\n1,2\n")
        doc.renameColumn(at: 0, to: "a")
        #expect(!um.canUndo)
    }

    @Test func emptyNameIsAllowed() async {
        let (doc, _) = await loaded("a,b\n1,2\n")
        doc.renameColumn(at: 0, to: "")
        #expect(doc.table?.headers == ["", "b"])
    }

    @Test func renameWithoutHeaderTurnsHeaderOnAndKeepsRows() async {
        let (doc, um) = await loaded("a,b\n1,2\n", header: false)
        doc.selection = GridSelection(columns: [0])
        #expect(doc.canRenameSelectedColumn)
        doc.renameColumn(at: 1, to: "x")
        #expect(doc.table?.headers == ["A", "x"])
        #expect(doc.table?.rowCount == 2)
        #expect(doc.configuration.firstRowAsHeader)
        #expect(um.undoActionName == "Rename Column")

        um.undo()
        #expect(doc.table?.headers == nil)
        #expect(doc.table?.rowCount == 2)
        #expect(!doc.configuration.firstRowAsHeader)
    }

    @Test func renameWithoutHeaderExportsHeaderLineFirst() async throws {
        let (doc, _) = await loaded("a,b\n1,2\n", header: false)
        doc.renameColumn(at: 1, to: "x")
        let snapshot = try doc.snapshot(contentType: .commaSeparatedText)
        let text = try String(decoding: CSVDocumentIO.serializedData(for: snapshot),
                              as: UTF8.self)
        let lines = text.split(whereSeparator: \.isNewline).map(String.init)
        #expect(lines == ["\"A\",\"x\"", "\"a\",\"b\"", "\"1\",\"2\""])
    }

    @Test func renameWithoutHeaderKeepsTheSort() async {
        let (doc, _) = await loaded("b,1\na,2\n", header: false)
        doc.sortRows(column: 0, direction: .ascending)
        let sort = doc.activeSort
        doc.renameColumn(at: 0, to: "key")
        #expect(doc.activeSort == sort)
        #expect(doc.table?.columns[0].cells == ["a", "b"])
    }

    @Test func renameWithoutHeaderToGeneratedNameRecordsNothing() async {
        let (doc, um) = await loaded("a,b\n1,2\n", header: false)
        doc.renameColumn(at: 0, to: "A")
        #expect(!um.canUndo)
        #expect(doc.table?.headers == nil)
    }

    @Test func undoAndRedoRestoreHeaderModeRowsAndGeneratedNames() async {
        let (doc, um) = await loaded("a,b\n1,2\n", header: false)
        let original = doc.table
        doc.renameColumn(at: 1, to: "x")
        let renamed = doc.table

        um.undo()
        #expect(doc.table == original)
        #expect(doc.table?.headers == nil)
        #expect(!doc.configuration.firstRowAsHeader)
        um.redo()
        #expect(doc.table == renamed)
        #expect(doc.table?.headers == ["A", "x"])
        #expect(doc.configuration.firstRowAsHeader)
    }

    @Test func undoRestoresSortAndInsertedRowsAndColumns() async {
        let (doc, um) = await loaded("b,1\na,2\n", header: false)
        let original = doc.table
        doc.sortRows(column: 0, direction: .ascending)
        um.undo()
        #expect(doc.table == original)
        #expect(doc.activeSort == nil)
        um.redo()
        #expect(doc.activeSort != nil)

        doc.addRowBelowSelection()
        doc.addColumnRightOfSelection()
        #expect(doc.table?.columns.count == 3)
        um.undo()
        #expect(doc.table?.columns.count == 2)
        um.undo()
        #expect(doc.table?.rowCount == 2)
    }

    @Test func selectedColumnRenameNeedsExactlyOneColumn() async {
        let (doc, _) = await loaded("a,b\n1,2\n")
        #expect(!doc.canRenameSelectedColumn)
        doc.selection = GridSelection(columns: [1])
        #expect(doc.canRenameSelectedColumn)
        doc.selection = GridSelection(columns: [0, 1])
        #expect(!doc.canRenameSelectedColumn)
    }

    @Test func renameSelectedColumnAsksTheGridToOpenTheEditor() async {
        let (doc, _) = await loaded("a,b\n1,2\n")
        let session = RecordingSession()
        doc.editingSession = session
        doc.renameSelectedColumn()
        #expect(session.renamedColumns.isEmpty)

        doc.selection = GridSelection(columns: [1])
        doc.renameSelectedColumn()
        #expect(session.renamedColumns == [1])
    }
}
