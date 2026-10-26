import AppKit
import Foundation
import TabToolKit
import Testing
@testable import TabTool

@Suite @MainActor struct GridCoordinatorTests {
    /// Attaches a coordinator to a standalone table view built from `document`.
    private func makeGrid(_ document: CSVDocument) -> (CSVGridCoordinator, CSVTableView) {
        let coordinator = CSVGridCoordinator(document: document)
        let tableView = CSVTableView()
        tableView.allowsMultipleSelection = true
        tableView.allowsColumnSelection = true
        coordinator.attach(to: tableView)
        update(coordinator, from: document)
        return (coordinator, tableView)
    }

    private func update(_ coordinator: CSVGridCoordinator, from document: CSVDocument) {
        coordinator.update(
            presenter: GridPresenter(table: document.table ?? .newDocument(),
                                     configuration: document.configuration),
            isEditable: true
        )
    }

    @Test func rowSelectionUpdatesDocument() {
        let document = CSVDocument()
        let (coordinator, tableView) = makeGrid(document)
        withExtendedLifetime(coordinator) {
            tableView.selectRowIndexes(IndexSet(integer: 0), byExtendingSelection: false)
        }
        #expect(document.selection == GridSelection(rows: IndexSet(integer: 0)))
    }

    @Test func columnSelectionUpdatesDocument() {
        let document = CSVDocument()
        let (coordinator, tableView) = makeGrid(document)
        withExtendedLifetime(coordinator) {
            tableView.selectColumnIndexes(IndexSet(integer: 1), byExtendingSelection: false)
        }
        #expect(document.selection == GridSelection(columns: IndexSet(integer: 1)))
    }

    @Test func externalSelectionIsAppliedToTheTable() {
        let document = CSVDocument()
        let (coordinator, tableView) = makeGrid(document)
        document.selection = GridSelection(rows: IndexSet(integer: 0))
        update(coordinator, from: document)
        #expect(tableView.selectedRowIndexes == IndexSet(integer: 0))
    }

    @Test func switchingSelectionToColumnsClearsRows() {
        let document = CSVDocument()
        let (coordinator, tableView) = makeGrid(document)
        tableView.selectRowIndexes(IndexSet(integer: 0), byExtendingSelection: false)

        document.selection = GridSelection(columns: IndexSet(integer: 1))
        update(coordinator, from: document)

        #expect(tableView.selectedRowIndexes.isEmpty)
        #expect(tableView.selectedColumnIndexes == IndexSet(integer: 1))
    }

    @Test func selectionSurvivesARowCountReload() {
        let document = CSVDocument()
        let (coordinator, tableView) = makeGrid(document)
        document.selection = GridSelection(rows: IndexSet(integer: 0))
        update(coordinator, from: document)

        document.addRow(at: 1)
        update(coordinator, from: document)

        #expect(document.table?.rowCount == 2)
        #expect(tableView.selectedRowIndexes == IndexSet(integer: 0))
    }

    @Test func fieldEditorRunsWithoutLocalUndo() {
        let document = CSVDocument()
        let (coordinator, _) = makeGrid(document)
        let fieldEditor = NSTextView()
        fieldEditor.allowsUndo = true
        coordinator.controlTextDidBeginEditing(
            Notification(name: NSText.didBeginEditingNotification, object: nil,
                         userInfo: ["NSFieldEditor": fieldEditor])
        )
        #expect(fieldEditor.allowsUndo == false)
    }

    @Test func endEditingFromDetachedCellIsIgnored() throws {
        let document = CSVDocument()
        let (coordinator, _) = makeGrid(document)
        let cell = CSVGridCellView()
        let field = try #require(cell.textField)
        field.stringValue = "stale"
        coordinator.controlTextDidEndEditing(
            Notification(name: NSControl.textDidEndEditingNotification, object: field)
        )
        #expect(document.table?.cell(row: 0, column: 0) == "")
    }

    @Test func endingCellEditingCommitsAndKeepsTheCellView() async throws {
        let undoManager = UndoManager()
        let document = CSVDocument(data: Data("a,b\nc,d\n".utf8),
                                   configuration: CSVConfiguration(),
                                   undoManager: undoManager)
        await document.load()
        let (coordinator, tableView) = makeGrid(document)

        let cell = try #require(tableView.view(atColumn: 0, row: 0,
                                               makeIfNecessary: true) as? CSVGridCellView)
        let field = try #require(cell.textField)
        field.stringValue = "changed"
        coordinator.controlTextDidEndEditing(
            Notification(name: NSControl.textDidEndEditingNotification, object: field)
        )

        #expect(document.table?.cell(row: 0, column: 0) == "changed")
        #expect(undoManager.undoActionName == "Edit Cell")

        update(coordinator, from: document)
        #expect(tableView.view(atColumn: 0, row: 0, makeIfNecessary: false) === cell)
    }
}
