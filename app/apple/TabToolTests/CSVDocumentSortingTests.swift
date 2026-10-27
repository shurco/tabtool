import Foundation
import Testing
import TabToolKit
@testable import TabTool

@Suite @MainActor struct CSVDocumentSortingTests {
    private final class OneShotSession: DocumentEditingSession {
        var onEndEditing: (() -> Void)?

        func endEditing() {
            let action = onEndEditing
            onEndEditing = nil
            action?()
        }

        func beginRenamingColumn(at column: Int) {}
    }

    private func loaded(_ csv: String) async -> (CSVDocument, UndoManager) {
        let undoManager = UndoManager()
        let doc = CSVDocument(data: Data(csv.utf8), configuration: .init(),
                              undoManager: undoManager)
        await doc.load()
        return (doc, undoManager)
    }

    private func column(_ doc: CSVDocument, _ index: Int) -> [String]? {
        doc.table?.columns[index].cells
    }

    @Test func sortThenUndoAndRedoRestoreRowsAndSortState() async {
        let (doc, um) = await loaded("b\na\nc\n")
        doc.sortRows(column: 0, direction: .ascending)
        #expect(column(doc, 0) == ["a", "b", "c"])
        #expect(doc.activeSort?.direction == .ascending)

        um.undo()
        #expect(column(doc, 0) == ["b", "a", "c"])
        #expect(doc.activeSort == nil)

        um.redo()
        #expect(column(doc, 0) == ["a", "b", "c"])
        #expect(doc.activeSort?.direction == .ascending)
    }

    @Test func ascendingDescendingRemoveRestoresSourceOrder() async {
        let (doc, _) = await loaded("b\na\nc\n")
        doc.sortRows(column: 0, direction: .ascending)
        doc.sortRows(column: 0, direction: .descending)
        #expect(column(doc, 0) == ["c", "b", "a"])

        doc.removeSort()
        #expect(column(doc, 0) == ["b", "a", "c"])
        #expect(doc.activeSort == nil)
        #expect(!doc.canRemoveSort)
    }

    @Test func cellEditKeepsSortAndRemoveSortKeepsTheEditedRow() async {
        let (doc, _) = await loaded("b,1\na,2\n")
        doc.sortRows(column: 0, direction: .ascending)
        doc.setCell("z", row: 0, column: 1)
        #expect(doc.activeSort != nil)

        doc.removeSort()
        #expect(column(doc, 0) == ["b", "a"])
        #expect(column(doc, 1) == ["1", "z"])
    }

    @Test func removeSortClearsStateWhenRowsLookIdentical() async {
        let (doc, _) = await loaded("b\na\n")
        doc.sortRows(column: 0, direction: .ascending)
        doc.setCell("b", row: 0, column: 0)
        doc.removeSort()
        #expect(doc.activeSort == nil)
        #expect(!doc.canRemoveSort)
    }

    @Test func resortingSameDirectionAfterEditReordersAndKeepsSource() async {
        let (doc, _) = await loaded("b\na\nc\n")
        doc.sortRows(column: 0, direction: .ascending)
        doc.setCell("z", row: 0, column: 0)
        doc.sortRows(column: 0, direction: .ascending)
        #expect(column(doc, 0) == ["b", "c", "z"])

        doc.removeSort()
        #expect(column(doc, 0) == ["b", "z", "c"])
    }

    @Test func sortingAnotherColumnComposesAndRemoveSortRestoresOriginal() async {
        let (doc, _) = await loaded("b,2\na,3\nc,1\n")
        doc.sortRows(column: 0, direction: .ascending)
        doc.sortRows(column: 1, direction: .ascending)
        #expect(column(doc, 1) == ["1", "2", "3"])

        doc.removeSort()
        #expect(column(doc, 0) == ["b", "a", "c"])
        #expect(doc.activeSort == nil)
    }

    @Test func removeSortUndoAndRedo() async {
        let (doc, um) = await loaded("b\na\nc\n")
        doc.sortRows(column: 0, direction: .ascending)
        doc.removeSort()

        um.undo()
        #expect(column(doc, 0) == ["a", "b", "c"])
        #expect(doc.activeSort?.direction == .ascending)

        um.redo()
        #expect(column(doc, 0) == ["b", "a", "c"])
        #expect(doc.activeSort == nil)
    }

    @Test func structuralChangeClearsSort() async {
        let (doc, _) = await loaded("b\na\n")
        doc.sortRows(column: 0, direction: .ascending)
        doc.addRow(at: 0)
        #expect(doc.activeSort == nil)

        let (other, _) = await loaded("b\na\n")
        other.sortRows(column: 0, direction: .ascending)
        other.setHeaderEnabled(true)
        #expect(other.activeSort == nil)
    }

    @Test func alreadySortedColumnSetsSortWithoutUndo() async {
        let (doc, um) = await loaded("a\nb\n")
        doc.sortRows(column: 0, direction: .ascending)
        #expect(doc.activeSort != nil)
        #expect(!um.canUndo)
    }

    @Test func cycleSortGoesAscendingDescendingThenRemoves() async {
        let (doc, _) = await loaded("b\na\nc\n")
        doc.cycleSort(column: 0)
        #expect(doc.sortDirection(forColumn: 0) == .ascending)
        #expect(column(doc, 0) == ["a", "b", "c"])
        doc.cycleSort(column: 0)
        #expect(doc.sortDirection(forColumn: 0) == .descending)
        #expect(column(doc, 0) == ["c", "b", "a"])
        doc.cycleSort(column: 0)
        #expect(doc.activeSort == nil)
        #expect(column(doc, 0) == ["b", "a", "c"])
    }

    @Test func cycleSortOnAnotherColumnStartsAscending() async {
        let (doc, _) = await loaded("b,1\na,2\n")
        doc.sortRows(column: 0, direction: .descending)
        doc.cycleSort(column: 1)
        #expect(doc.sortDirection(forColumn: 1) == .ascending)
        #expect(doc.sortDirection(forColumn: 0) == nil)
    }

    @Test func sortCommitsTheFieldEditorFirst() async {
        let (doc, _) = await loaded("b\na\n")
        let session = OneShotSession()
        doc.editingSession = session
        session.onEndEditing = { doc.setCell("0", row: 1, column: 0) }

        doc.sortRows(column: 0, direction: .ascending)
        #expect(column(doc, 0) == ["0", "b"])
    }

    @Test func canSortSelectionNeedsExactlyOneColumn() async {
        let (doc, _) = await loaded("a,b\n1,2\n")
        #expect(!doc.canSortSelection)
        doc.selection = GridSelection(columns: [0])
        #expect(doc.canSortSelection)
        doc.selection = GridSelection(columns: [0, 1])
        #expect(!doc.canSortSelection)
        doc.selection = GridSelection(rows: [0])
        #expect(!doc.canSortSelection)
    }
}
