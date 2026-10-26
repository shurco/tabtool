import AppKit
import Foundation
import TabToolKit
import Testing
@testable import TabTool

@Suite @MainActor struct GridCoordinatorTests {
    /// Attaches a coordinator to a standalone table view built from `document`.
    private func makeGrid(
        _ document: CSVDocument
    ) -> (CSVGridCoordinator, CSVTableView, CSVTableHeaderView) {
        let coordinator = CSVGridCoordinator(document: document)
        let tableView = CSVTableView()
        let headerView = CSVTableHeaderView()
        tableView.headerView = headerView
        tableView.allowsMultipleSelection = true
        coordinator.attach(to: tableView)
        update(coordinator, from: document)
        return (coordinator, tableView, headerView)
    }

    private func update(_ coordinator: CSVGridCoordinator, from document: CSVDocument) {
        coordinator.update(
            presenter: GridPresenter(table: document.table ?? .newDocument(),
                                     configuration: document.configuration),
            isEditable: true
        )
    }

    @Test func rowSelectionUpdatesDocument() async {
        let document = await GridTestSupport.makeLoadedNewDocument()
        let (coordinator, tableView, _) = makeGrid(document)
        withExtendedLifetime(coordinator) {
            tableView.selectRowIndexes(IndexSet(integer: 0), byExtendingSelection: false)
        }
        #expect(document.selection == GridSelection(rows: IndexSet(integer: 0)))
    }

    @Test func headerClickSelectsColumnAndHighlightsIt() async throws {
        let document = await GridTestSupport.makeLoadedNewDocument()
        let (coordinator, tableView, headerView) = makeGrid(document)
        withExtendedLifetime(coordinator) {
            headerView.clickColumn(at: 1, modifiers: [])
        }

        #expect(!tableView.allowsColumnSelection)
        #expect(document.selection == GridSelection(columns: IndexSet(integer: 1)))
        #expect(headerView.selectedColumns == IndexSet(integer: 1))

        let selectedCell = try #require(
            tableView.view(atColumn: 1, row: 0, makeIfNecessary: true)
                as? CSVGridCellView
        )
        let plainCell = try #require(
            tableView.view(atColumn: 0, row: 0, makeIfNecessary: true)
                as? CSVGridCellView
        )
        #expect(selectedCell.isColumnSelected)
        #expect(!plainCell.isColumnSelected)
    }

    @Test func headerClickClearsRowSelection() async throws {
        let document = await GridTestSupport.makeLoadedNewDocument()
        let (coordinator, tableView, headerView) = makeGrid(document)
        withExtendedLifetime(coordinator) {
            tableView.selectRowIndexes(IndexSet(integer: 0), byExtendingSelection: false)
            headerView.clickColumn(at: 1, modifiers: [])
        }

        #expect(document.selection == GridSelection(columns: IndexSet(integer: 1)))
        #expect(tableView.selectedRowIndexes.isEmpty)
        #expect(headerView.selectedColumns == IndexSet(integer: 1))
    }

    @Test func rowSelectionClearsColumnHighlight() async throws {
        let document = await GridTestSupport.makeLoadedNewDocument()
        let (coordinator, tableView, headerView) = makeGrid(document)
        withExtendedLifetime(coordinator) {
            headerView.clickColumn(at: 1, modifiers: [])
            tableView.selectRowIndexes(IndexSet(integer: 0), byExtendingSelection: false)
        }

        #expect(document.selection == GridSelection(rows: IndexSet(integer: 0)))
        #expect(headerView.selectedColumns.isEmpty)

        let cell = try #require(
            tableView.view(atColumn: 1, row: 0, makeIfNecessary: true)
                as? CSVGridCellView
        )
        #expect(!cell.isColumnSelected)
    }

    @Test func commandClickTogglesColumns() async {
        let document = await GridTestSupport.makeLoadedNewDocument()
        let (coordinator, _, headerView) = makeGrid(document)
        withExtendedLifetime(coordinator) {
            headerView.clickColumn(at: 0, modifiers: [])
            headerView.clickColumn(at: 2, modifiers: .command)
        }
        #expect(document.selection == GridSelection(columns: IndexSet([0, 2])))
        #expect(headerView.selectedColumns == IndexSet([0, 2]))

        withExtendedLifetime(coordinator) {
            headerView.clickColumn(at: 0, modifiers: .command)
        }
        #expect(document.selection == GridSelection(columns: IndexSet(integer: 2)))
    }

    @Test func shiftClickExtendsColumnRange() async {
        let document = await GridTestSupport.makeLoadedNewDocument()
        let (coordinator, _, headerView) = makeGrid(document)
        withExtendedLifetime(coordinator) {
            headerView.clickColumn(at: 0, modifiers: [])
            headerView.clickColumn(at: 2, modifiers: .shift)
        }
        #expect(document.selection == GridSelection(columns: IndexSet([0, 1, 2])))
        #expect(headerView.selectedColumns == IndexSet([0, 1, 2]))
    }

    @Test func externalSelectionIsAppliedToTheTable() async {
        let document = await GridTestSupport.makeLoadedNewDocument()
        let (coordinator, tableView, headerView) = makeGrid(document)
        document.selection = GridSelection(rows: IndexSet(integer: 0))
        update(coordinator, from: document)
        #expect(tableView.selectedRowIndexes == IndexSet(integer: 0))
        #expect(headerView.selectedColumns.isEmpty)
    }

    @Test func externalColumnSelectionHighlightsHeaderAndCell() async throws {
        let document = await GridTestSupport.makeLoadedNewDocument()
        let (coordinator, tableView, headerView) = makeGrid(document)
        document.selection = GridSelection(columns: IndexSet(integer: 1))
        update(coordinator, from: document)

        #expect(tableView.selectedRowIndexes.isEmpty)
        #expect(headerView.selectedColumns == IndexSet(integer: 1))

        let cell = try #require(
            tableView.view(atColumn: 1, row: 0, makeIfNecessary: true)
                as? CSVGridCellView
        )
        #expect(cell.isColumnSelected)
    }

    @Test func switchingSelectionToColumnsClearsRows() async {
        let document = await GridTestSupport.makeLoadedNewDocument()
        let (coordinator, tableView, headerView) = makeGrid(document)
        tableView.selectRowIndexes(IndexSet(integer: 0), byExtendingSelection: false)

        document.selection = GridSelection(columns: IndexSet(integer: 1))
        update(coordinator, from: document)

        #expect(tableView.selectedRowIndexes.isEmpty)
        #expect(headerView.selectedColumns == IndexSet(integer: 1))
        #expect(document.selection == GridSelection(columns: IndexSet(integer: 1)))
    }

    @Test func selectionSurvivesARowCountReload() async {
        let document = await GridTestSupport.makeLoadedNewDocument()
        let (coordinator, tableView, _) = makeGrid(document)
        document.selection = GridSelection(rows: IndexSet(integer: 0))
        update(coordinator, from: document)

        document.addRow(at: 1)
        update(coordinator, from: document)

        #expect(document.table?.rowCount == 2)
        #expect(tableView.selectedRowIndexes == IndexSet(integer: 0))
    }

    @Test func fieldEditorRunsWithoutLocalUndo() async {
        let document = await GridTestSupport.makeLoadedNewDocument()
        let (coordinator, _, _) = makeGrid(document)
        let fieldEditor = NSTextView()
        fieldEditor.allowsUndo = true
        coordinator.controlTextDidBeginEditing(
            Notification(name: NSText.didBeginEditingNotification, object: nil,
                         userInfo: ["NSFieldEditor": fieldEditor])
        )
        #expect(fieldEditor.allowsUndo == false)
    }

    @Test func endEditingFromDetachedCellIsIgnored() async throws {
        let document = await GridTestSupport.makeLoadedNewDocument()
        let (coordinator, _, _) = makeGrid(document)
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
        let (coordinator, tableView, _) = makeGrid(document)

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
