import Foundation
import Testing
import TabToolKit
@testable import TabTool

@Suite @MainActor struct CSVDocumentMovingTests {
    private func loaded(_ csv: String, configuration: CSVConfiguration = .init())
        async -> (CSVDocument, UndoManager) {
        let undoManager = UndoManager()
        let doc = CSVDocument(data: Data(csv.utf8), configuration: configuration,
                              undoManager: undoManager)
        await doc.load()
        return (doc, undoManager)
    }

    @Test func moveRowsDownKeepsGapsAndSelection() async {
        let (doc, um) = await loaded("0\n1\n2\n3\n4\n")
        doc.selection = GridSelection(rows: [1, 3])
        doc.moveSelectedRows(by: 1)
        #expect(doc.table?.columns[0].cells == ["0", "2", "1", "4", "3"])
        #expect(doc.selection.rows == [2, 4])

        um.undo()
        #expect(doc.table?.columns[0].cells == ["0", "1", "2", "3", "4"])
    }

    @Test func moveColumnsLeftMovesHeadersAndSelection() async {
        let (doc, _) = await loaded("A,B,C\n1,2,3\n",
                                    configuration: CSVConfiguration(firstRowAsHeader: true))
        doc.selection = GridSelection(columns: [1, 2])
        doc.moveSelectedColumns(by: -1)
        #expect(doc.table?.headers == ["B", "C", "A"])
        #expect(doc.selection.columns == [0, 1])
    }

    @Test func movesAreDisabledAtEdgesAndWithoutSelection() async {
        let (doc, _) = await loaded("a,b,c\n1,2,3\n4,5,6\n")
        #expect(!doc.canMoveSelectedRows(by: 1))
        #expect(!doc.canMoveSelectedColumns(by: -1))

        doc.selection = GridSelection(rows: [0, 2])
        #expect(!doc.canMoveSelectedRows(by: -1))
        #expect(!doc.canMoveSelectedRows(by: 1))

        doc.selection = GridSelection(columns: [0])
        #expect(!doc.canMoveSelectedColumns(by: -1))
        #expect(doc.canMoveSelectedColumns(by: 1))
    }

    @Test func moveRowsClearsSort() async {
        let (doc, _) = await loaded("b\na\nc\n")
        doc.sortRows(column: 0, direction: .ascending)
        doc.selection = GridSelection(rows: [0])
        doc.moveSelectedRows(by: 1)
        #expect(doc.activeSort == nil)
    }

    @Test func columnMovesKeepTheSort() async {
        let (doc, um) = await loaded("b,1\na,2\n")
        doc.sortRows(column: 0, direction: .ascending)
        let sort = doc.activeSort
        #expect(sort != nil)

        doc.moveColumn(from: 0, to: 1)
        #expect(doc.activeSort == sort)
        #expect(doc.canRemoveSort)

        doc.selection = GridSelection(columns: [1])
        doc.moveSelectedColumns(by: -1)
        #expect(doc.activeSort == sort)

        um.undo()
        #expect(doc.activeSort == sort)
        #expect(doc.table?.columns[0].cells == ["2", "1"])

        doc.removeSort()
        #expect(doc.table?.columns[1].cells == ["b", "a"])
    }
}
