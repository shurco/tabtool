import AppKit
import Foundation
import Testing
import TabToolKit
@testable import TabTool

/// The grid shows a filtered row subset but keeps every document index in
/// model rows.
@Suite @MainActor struct GridFilteringTests {
    /// A grid over four rows filtered to the rows whose first column is "a"
    /// (model rows 0 and 2).
    private func filteredGrid(
        undoManager: UndoManager? = nil
    ) async throws -> (CSVDocument, CSVGridCoordinator, CSVTableView) {
        let doc = CSVDocument(data: Data("a,1\nb,2\na,3\nc,4\n".utf8), configuration: .init(),
                              undoManager: undoManager)
        await doc.load()
        let (coordinator, tableView, _) = GridTestSupport.makeGrid(doc)
        let columnID = try #require(doc.table?.columnIDs[0])
        doc.rowFilter.criteria = [FilterCriterion(columnID: columnID, op: .equals, value: "a")]
        doc.applyAllFilters()
        GridTestSupport.update(coordinator, from: doc)
        return (doc, coordinator, tableView)
    }

    private func text(_ tableView: CSVTableView, row: Int, column: Int = 1) -> String? {
        let cell = tableView.view(atColumn: column, row: row, makeIfNecessary: true) as? CSVGridCellView
        return cell?.textField?.stringValue
    }

    @Test func tableShowsOnlyTheVisibleRows() async throws {
        let (_, coordinator, tableView) = try await filteredGrid()
        withExtendedLifetime(coordinator) {
            #expect(tableView.numberOfRows == 2)
            #expect(text(tableView, row: 0) == "1")
            #expect(text(tableView, row: 1) == "3")
        }
    }

    @Test func clearingTheFilterShowsEveryRowAgain() async throws {
        let (doc, coordinator, tableView) = try await filteredGrid()
        doc.rowFilter.clear()
        GridTestSupport.update(coordinator, from: doc)
        withExtendedLifetime(coordinator) {
            #expect(tableView.numberOfRows == 4)
        }
    }

    @Test func changingTheVisibleSetWithTheSameCountReloadsTheRows() async throws {
        let (doc, coordinator, tableView) = try await filteredGrid()
        let columnID = try #require(doc.table?.columnIDs[0])
        doc.rowFilter.criteria = [FilterCriterion(columnID: columnID, op: .notEquals, value: "a")]
        doc.applyAllFilters()
        #expect(doc.rowFilter.visibleRows == [1, 3])
        GridTestSupport.update(coordinator, from: doc)
        withExtendedLifetime(coordinator) {
            #expect(text(tableView, row: 0) == "2")
            #expect(text(tableView, row: 1) == "4")
        }
    }

    @Test func editingAFilteredRowWritesTheMatchingModelRow() async throws {
        let (doc, coordinator, tableView) = try await filteredGrid()
        let cell = try #require(tableView.view(atColumn: 1, row: 1,
                                               makeIfNecessary: true) as? CSVGridCellView)
        let field = try #require(cell.textField)
        field.stringValue = "edited"
        coordinator.controlTextDidEndEditing(
            Notification(name: NSControl.textDidEndEditingNotification, object: field)
        )

        #expect(doc.table?.columns[1].cells == ["1", "2", "edited", "4"])
        // The edit does not re-evaluate the filter.
        #expect(doc.rowFilter.visibleRows == [0, 2])
    }

    @Test func tableSelectionIsStoredAsModelRows() async throws {
        let (doc, coordinator, tableView) = try await filteredGrid()
        withExtendedLifetime(coordinator) {
            tableView.selectRowIndexes(IndexSet(integer: 1), byExtendingSelection: false)
        }
        #expect(doc.selection == GridSelection(rows: IndexSet(integer: 2)))
    }

    @Test func documentSelectionShowsAsDisplayRows() async throws {
        let (doc, coordinator, tableView) = try await filteredGrid()
        doc.selection = GridSelection(rows: IndexSet([2]))
        GridTestSupport.update(coordinator, from: doc)
        withExtendedLifetime(coordinator) {
            #expect(tableView.selectedRowIndexes == IndexSet(integer: 1))
            #expect(doc.selection == GridSelection(rows: IndexSet(integer: 2)))
        }
    }

    @Test func selectAllSelectsOnlyVisibleRows() async throws {
        let (doc, coordinator, tableView) = try await filteredGrid()
        withExtendedLifetime(coordinator) {
            coordinator.selectAllRows()
            #expect(tableView.selectedRowIndexes == IndexSet([0, 1]))
        }
        #expect(doc.selection == GridSelection(rows: IndexSet([0, 2])))
    }

    @Test func copyTakesTheSelectedModelRows() async throws {
        let (_, coordinator, tableView) = try await filteredGrid()
        let pasteboard = GridTestSupport.makePasteboard()
        withExtendedLifetime(coordinator) {
            tableView.selectRowIndexes(IndexSet(integer: 1), byExtendingSelection: false)
            #expect(coordinator.copySelection(to: pasteboard))
        }
        #expect(pasteboard.string(forType: .string) == "a\t3")
    }

    @Test func rowStructureEditsAreDisabledWhileFiltering() async throws {
        let (doc, coordinator, tableView) = try await filteredGrid()
        doc.selection = GridSelection(rows: IndexSet([0]))
        let pasteboard = GridTestSupport.makePasteboard()
        pasteboard.clearContents()
        pasteboard.setString("x", forType: .string)

        withExtendedLifetime(coordinator) {
            #expect(!coordinator.allowsRowStructuralEdits)
            #expect(!coordinator.canPaste)
            #expect(!coordinator.paste(from: pasteboard))
            #expect(!coordinator.canDeleteSelection)
            #expect(!coordinator.deleteSelection())
            #expect(coordinator.tableView(tableView, pasteboardWriterForRow: 0) == nil)
            #expect(!coordinator.moveRowsForDrop(IndexSet([0]), to: 2))
            #expect(coordinator.dropOperation(proposed: .above, source: tableView,
                                              pasteboard: pasteboard).isEmpty)
            #expect(coordinator.rowMenu().items.allSatisfy { !$0.isEnabled })
        }
        #expect(doc.table?.rowCount == 4)
    }

    @Test func rowStructureEditsReturnOnceTheFilterIsCleared() async throws {
        let (doc, coordinator, _) = try await filteredGrid()
        doc.rowFilter.clear()
        withExtendedLifetime(coordinator) {
            #expect(coordinator.allowsRowStructuralEdits)
            #expect(coordinator.rowMenu().items.allSatisfy { $0.isEnabled })
        }
    }

    @Test func columnStructureStaysEditableWhileFiltering() async throws {
        let (_, coordinator, _) = try await filteredGrid()
        withExtendedLifetime(coordinator) {
            #expect(coordinator.allowsStructuralEdits)
            #expect(coordinator.headerMenu(forColumn: 0).items.first?.isEnabled == true)
        }
    }
}
