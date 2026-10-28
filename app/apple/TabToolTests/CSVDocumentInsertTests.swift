import Foundation
import Testing
import TabToolKit
@testable import TabTool

@Suite @MainActor struct CSVDocumentInsertTests {
    private func loaded(_ csv: String) async -> CSVDocument {
        let doc = CSVDocument(data: Data(csv.utf8), configuration: .init(), undoManager: UndoManager())
        await doc.load()
        return doc
    }

    @Test func insertingBeforeTheSelectionNeedsASelection() async {
        let doc = await loaded("a,1\nb,2\n")
        #expect(!doc.canAddRowAboveSelection)
        #expect(!doc.canAddColumnLeftOfSelection)

        doc.selection = GridSelection(rows: IndexSet([1]))
        #expect(doc.canAddRowAboveSelection)
        #expect(!doc.canAddColumnLeftOfSelection)

        doc.selection = GridSelection(columns: IndexSet([1]))
        #expect(!doc.canAddRowAboveSelection)
        #expect(doc.canAddColumnLeftOfSelection)
    }

    @Test func insertingAfterWithoutASelectionAppendsAtTheEnd() async {
        let doc = await loaded("a,1\nb,2\n")
        doc.addRowBelowSelection()
        #expect(doc.table?.rowCount == 3)
        #expect(doc.table?.cell(row: 2, column: 0) == "")
        #expect(doc.table?.cell(row: 1, column: 0) == "b")

        doc.selection = GridSelection()
        doc.addColumnRightOfSelection()
        #expect(doc.table?.columns.count == 3)
        #expect(doc.table?.cell(row: 0, column: 2) == "")
        #expect(doc.table?.cell(row: 0, column: 1) == "1")
    }

    @Test func insertingBeforeTheSelectionPlacesTheNewRowAndColumnFirst() async {
        let doc = await loaded("a,1\nb,2\n")
        doc.selection = GridSelection(rows: IndexSet([1]))
        doc.addRowAboveSelection()
        #expect(doc.table?.cell(row: 1, column: 0) == "")
        #expect(doc.table?.cell(row: 2, column: 0) == "b")

        doc.selection = GridSelection(columns: IndexSet([1]))
        doc.addColumnLeftOfSelection()
        #expect(doc.table?.cell(row: 0, column: 1) == "")
        #expect(doc.table?.cell(row: 0, column: 2) == "1")
    }
}
