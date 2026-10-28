import Combine
import Foundation
import Testing
import TabToolKit
@testable import TabTool

@Suite @MainActor struct CSVDocumentFilteringTests {
    /// Stands in for the grid's field editor: counts the commits it is asked for.
    private final class CountingSession: DocumentEditingSession {
        private(set) var endCount = 0
        func endEditing() { endCount += 1 }
        func beginRenamingColumn(at column: Int) {}
    }

    private func loaded(_ csv: String, fileURL: URL? = nil) async -> (CSVDocument, UndoManager) {
        let undoManager = UndoManager()
        let doc = CSVDocument(data: Data(csv.utf8), fileURL: fileURL, configuration: .init(),
                              undoManager: undoManager)
        await doc.load()
        return (doc, undoManager)
    }

    /// Applies "first column equals `value`" through the document.
    private func applyFilter(_ doc: CSVDocument, value: String, column: Int = 0) throws {
        let columnID = try #require(doc.table?.columnIDs[column])
        doc.rowFilter.criteria = [FilterCriterion(columnID: columnID, op: .equals, value: value)]
        doc.applyAllFilters()
    }

    @Test func applyAllFiltersEvaluatesTheCurrentTable() async throws {
        let (doc, _) = await loaded("a,1\nb,2\na,3\n")
        try applyFilter(doc, value: "a")
        #expect(doc.rowFilter.visibleRows == [0, 2])
        #expect(doc.rowFilter.isFiltering)
    }

    @Test func applyFilterUsesOnlyTheGivenCriterion() async throws {
        let (doc, _) = await loaded("a,1\nb,2\na,3\n")
        let ids = try #require(doc.table?.columnIDs)
        let first = FilterCriterion(columnID: ids[0], op: .equals, value: "a")
        let second = FilterCriterion(columnID: ids[1], op: .equals, value: "2")
        doc.rowFilter.criteria = [first, second]
        doc.applyFilter(criterionID: second.id)
        #expect(doc.rowFilter.visibleRows == [1])
    }

    @Test func applyingDropsSelectedRowsThatAreHidden() async throws {
        let (doc, _) = await loaded("a\nb\na\n")
        doc.selection = GridSelection(rows: IndexSet([0, 1, 2]))
        try applyFilter(doc, value: "a")
        #expect(doc.selection == GridSelection(rows: IndexSet([0, 2])))
    }

    @Test func visibleTableKeepsOnlyTheFilteredRowsWithTheirEdits() async throws {
        let (doc, _) = await loaded("a,1\nb,2\na,3\n")
        doc.addColumnRightOfSelection()
        try applyFilter(doc, value: "a")
        doc.setCell("z", row: 2, column: 1)

        let table = try #require(doc.visibleTableAfterEndingEditing())
        #expect(table.rowCount == 2)
        #expect(table.columns.count == 3)
        #expect(table.cell(row: 0, column: 1) == "1")
        #expect(table.cell(row: 1, column: 1) == "z")
    }

    @Test func visibleTableIsTheWholeTableWithoutAFilter() async throws {
        let (doc, _) = await loaded("a\nb\n")
        #expect(doc.visibleTableAfterEndingEditing() == doc.table)
    }

    @Test func visibleTableCommitsTheCellBeingEdited() async {
        let (doc, _) = await loaded("a\nb\n")
        let session = CountingSession()
        doc.editingSession = session
        _ = doc.visibleTableAfterEndingEditing()
        #expect(session.endCount == 1)
    }

    @Test func cellEditDoesNotReevaluateTheFilter() async throws {
        let (doc, _) = await loaded("a\nb\na\n")
        try applyFilter(doc, value: "a")
        doc.setCell("z", row: 0, column: 0)
        #expect(doc.rowFilter.visibleRows == [0, 2])
    }

    @Test func undoAndRedoOfACellEditReevaluateTheFilter() async throws {
        let (doc, um) = await loaded("a\nb\na\n")
        try applyFilter(doc, value: "a")
        doc.setCell("a", row: 1, column: 0)
        doc.setCell("z", row: 0, column: 0)
        // Cell edits keep the snapshot, so the rows have not changed yet.
        #expect(doc.rowFilter.visibleRows == [0, 2])

        // Undoing the second edit gives "a,a,a"; only a refresh shows row 1.
        um.undo()
        #expect(doc.rowFilter.visibleRows == [0, 1, 2])
        um.undo()
        #expect(doc.rowFilter.visibleRows == [0, 2])
        um.redo()
        #expect(doc.rowFilter.visibleRows == [0, 1, 2])
    }

    @Test func sortReevaluatesTheFilterAndUndoRestoresIt() async throws {
        let (doc, um) = await loaded("a,2\nb,1\na,3\n")
        try applyFilter(doc, value: "a")
        #expect(doc.rowFilter.visibleRows == [0, 2])

        doc.sortRows(column: 1, direction: .ascending)
        #expect(doc.table?.columns[1].cells == ["1", "2", "3"])
        #expect(doc.rowFilter.visibleRows == [1, 2])

        um.undo()
        #expect(doc.rowFilter.visibleRows == [0, 2])
        um.redo()
        #expect(doc.rowFilter.visibleRows == [1, 2])
    }

    @Test func undoOfRowDeletionKeepsVisibleRowsInRange() async throws {
        let (doc, um) = await loaded("a\nb\na\na\n")
        doc.deleteRows(at: IndexSet([3]))
        try applyFilter(doc, value: "a")
        #expect(doc.rowFilter.visibleRows == [0, 2])

        um.undo()
        #expect(doc.table?.rowCount == 4)
        #expect(doc.rowFilter.visibleRows == [0, 2, 3])
        um.redo()
        #expect(doc.rowFilter.visibleRows == [0, 2])
    }

    @Test func revertToSavedReevaluatesTheFilter() async throws {
        let url = FileManager.default.temporaryDirectory
            .appendingPathComponent("filter-revert-\(UUID()).csv")
        try Data("a\nb\na\n".utf8).write(to: url)
        defer { try? FileManager.default.removeItem(at: url) }
        let (doc, _) = await loaded("a\nb\na\n", fileURL: url)
        doc.setCell("a", row: 1, column: 0)
        try applyFilter(doc, value: "a")
        #expect(doc.rowFilter.visibleRows == [0, 1, 2])

        try await doc.revertToSaved()
        #expect(doc.table?.cell(row: 1, column: 0) == "b")
        #expect(doc.rowFilter.visibleRows == [0, 2])
    }

    @Test func deletingAColumnDropsItsCriteria() async throws {
        let (doc, _) = await loaded("a,1\nb,2\na,3\n")
        let ids = try #require(doc.table?.columnIDs)
        doc.rowFilter.criteria = [
            FilterCriterion(columnID: ids[0], op: .equals, value: "a"),
            FilterCriterion(columnID: ids[1], op: .greaterThan, value: "1")
        ]
        doc.applyAllFilters()
        #expect(doc.rowFilter.visibleRows == [2])

        doc.deleteColumns(at: IndexSet([1]))
        #expect(doc.rowFilter.appliedFilter?.criteria.map(\.columnID) == [ids[0]])
        #expect(doc.rowFilter.criteria.map(\.columnID) == [ids[0], nil])
        #expect(doc.rowFilter.visibleRows == [0, 2])
    }

    @Test func rowStructureIsLockedOnlyWhileFiltering() async throws {
        let (doc, _) = await loaded("a\nb\na\n")
        doc.selection = GridSelection(rows: IndexSet([0]))
        #expect(doc.canEditRowStructure)
        #expect(doc.canInsertRows)
        #expect(doc.canDeleteSelectedRows)
        #expect(doc.canMoveSelectedRows(by: 1))

        try applyFilter(doc, value: "a")
        #expect(!doc.canEditRowStructure)
        #expect(!doc.canInsertRows)
        #expect(!doc.canDeleteSelectedRows)
        #expect(!doc.canDeleteSelection)
        #expect(!doc.canMoveSelectedRows(by: 1))

        doc.addRowBelowSelection()
        doc.pasteTSV("x")
        #expect(doc.table?.rowCount == 3)

        doc.rowFilter.clear()
        #expect(doc.canEditRowStructure)
        #expect(doc.canInsertRows)
    }

    @Test func columnDeletionStaysAvailableWhileFiltering() async throws {
        let (doc, _) = await loaded("a,1\nb,2\n")
        try applyFilter(doc, value: "a")
        doc.selection = GridSelection(columns: IndexSet([1]))
        #expect(doc.canDeleteSelection)
        #expect(doc.canSortSelection)
    }

    @Test func columnCopyCoversOnlyVisibleRows() async throws {
        let (doc, _) = await loaded("a,1\nb,2\na,3\n")
        try applyFilter(doc, value: "a")
        doc.selection = GridSelection(columns: IndexSet([1]))
        #expect(doc.copySelection() == "1\n3")
    }

    @Test func largeModeCannotFilter() {
        #expect(!CSVDocument(largeFileURL: nil).canFilter)
    }

    @Test func memoryModeCanFilterOnceLoaded() async {
        let (doc, _) = await loaded("a\n")
        #expect(doc.canFilter)
        #expect(!CSVDocument(data: Data("a\n".utf8)).canFilter)
    }

    @Test func changesOfTheAppliedFilterRepublishFromTheDocument() async throws {
        let (doc, _) = await loaded("a\nb\n")
        var published = 0
        let observation = doc.objectWillChange.sink { published += 1 }
        defer { observation.cancel() }

        try applyFilter(doc, value: "a")
        #expect(published > 0)
    }

    @Test func toggleFilterBarRepublishesTheDocumentAndKeepsTheFilter() async throws {
        let (doc, _) = await loaded("a\nb\n")
        try applyFilter(doc, value: "a")
        var published = 0
        let observation = doc.objectWillChange.sink { published += 1 }
        defer { observation.cancel() }

        doc.toggleFilterBar()
        #expect(doc.rowFilter.isBarVisible)
        #expect(published > 0)
        doc.toggleFilterBar()
        #expect(!doc.rowFilter.isBarVisible)
        #expect(doc.rowFilter.isFiltering)
    }

    @Test func largeModeCannotToggleTheFilterBar() {
        let doc = CSVDocument(largeFileURL: nil)
        doc.toggleFilterBar()
        #expect(!doc.rowFilter.isBarVisible)
    }

    @Test func clearFilterCommitsTheCellBeingEdited() async throws {
        let (doc, _) = await loaded("a\nb\n")
        try applyFilter(doc, value: "a")
        let session = CountingSession()
        doc.editingSession = session

        doc.clearFilter()
        #expect(session.endCount == 1)
        #expect(!doc.rowFilter.isFiltering)
        #expect(doc.rowFilter.visibleRows == nil)
    }
}
