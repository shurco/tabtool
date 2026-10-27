import AppKit
import Foundation
import TabToolKit
import Testing
@testable import TabTool

@Suite @MainActor struct GridDragColumnTests {
    // MARK: - Row drag

    @Test func rowDragItemCarriesTheRowIndexAndTSV() async throws {
        let doc = CSVDocument(data: Data("1,2\n3,4\n".utf8), configuration: .init())
        await doc.load()
        let (coordinator, tableView, _) = GridTestSupport.makeGrid(doc)

        let item = try #require(
            coordinator.tableView(tableView, pasteboardWriterForRow: 1)
                as? NSPasteboardItem
        )
        #expect(item.string(forType: GridPasteboard.rowIndexes) == "1")
        #expect(item.string(forType: .tabularText) == "3\t4")
        #expect(item.string(forType: .string) == "3\t4")
    }

    @Test func rowDragCarriesTheWholeSelectionAsOneTSVBlock() async throws {
        let doc = CSVDocument(data: Data("a,b\nc,d\ne,f\n".utf8), configuration: .init())
        await doc.load()
        doc.selection = GridSelection(rows: IndexSet([0, 2]))
        let (coordinator, tableView, _) = GridTestSupport.makeGrid(doc)

        let first = try #require(
            coordinator.tableView(tableView, pasteboardWriterForRow: 0)
                as? NSPasteboardItem
        )
        let second = try #require(
            coordinator.tableView(tableView, pasteboardWriterForRow: 2)
                as? NSPasteboardItem
        )

        #expect(first.string(forType: .tabularText) == "a\tb\ne\tf")
        #expect(first.string(forType: .string) == "a\tb\ne\tf")
        #expect(second.string(forType: .string) == nil)
        #expect(first.string(forType: GridPasteboard.rowIndexes) == "0,2")
        #expect(second.string(forType: GridPasteboard.rowIndexes) == "0,2")
    }

    @Test func rowDragIgnoresInvalidRows() async throws {
        let doc = CSVDocument(data: Data("a\n".utf8), configuration: .init())
        await doc.load()
        let (coordinator, tableView, _) = GridTestSupport.makeGrid(doc)

        #expect(coordinator.tableView(tableView, pasteboardWriterForRow: 5) == nil)
    }

    @Test func rowDropReadsEveryDraggedIndex() {
        let doc = CSVDocument()
        let (coordinator, _, _) = GridTestSupport.makeGrid(doc)
        let pasteboard = GridTestSupport.makePasteboard()
        pasteboard.clearContents()
        let first = NSPasteboardItem()
        first.setString("0", forType: GridPasteboard.rowIndexes)
        let second = NSPasteboardItem()
        second.setString("2", forType: GridPasteboard.rowIndexes)
        pasteboard.writeObjects([first, second])

        #expect(coordinator.rowIndexes(from: pasteboard) == IndexSet([0, 2]))
    }

    @Test func rowDropMovesRowsAndSelectsTheMovedOnes() async throws {
        let um = UndoManager()
        let doc = CSVDocument(data: Data("a\nb\nc\n".utf8), configuration: .init(),
                              undoManager: um)
        await doc.load()
        let (coordinator, tableView, _) = GridTestSupport.makeGrid(doc)

        coordinator.moveRowsForDrop(IndexSet([0, 2]), to: 3)

        #expect(doc.table?.columns[0].cells == ["b", "a", "c"])
        #expect(doc.selection == GridSelection(rows: IndexSet([1, 2])))
        #expect(um.undoActionName == "Move Rows")

        GridTestSupport.update(coordinator, from: doc)
        #expect(tableView.selectedRowIndexes == IndexSet([1, 2]))
    }

    @Test func dropAcceptsOnlyLocalAboveRowDrags() {
        let doc = CSVDocument()
        let (coordinator, tableView, _) = GridTestSupport.makeGrid(doc)
        let pasteboard = GridTestSupport.makePasteboard()
        pasteboard.clearContents()
        let item = NSPasteboardItem()
        item.setString("0", forType: GridPasteboard.rowIndexes)
        pasteboard.writeObjects([item])

        #expect(coordinator.dropOperation(proposed: .above, source: tableView,
                                          pasteboard: pasteboard) == .move)
        #expect(coordinator.dropOperation(proposed: .on, source: tableView,
                                          pasteboard: pasteboard) == [])
        #expect(coordinator.dropOperation(proposed: .above, source: NSTextField(),
                                          pasteboard: pasteboard) == [])

        let foreign = GridTestSupport.makePasteboard()
        foreign.clearContents()
        foreign.setString("a", forType: .string)
        #expect(coordinator.dropOperation(proposed: .above, source: tableView,
                                          pasteboard: foreign) == [])
    }

    // MARK: - Column reorder

    @Test func movingATableColumnReordersTheDocument() async throws {
        let doc = CSVDocument(data: Data("a,b,c\n1,2,3\n".utf8), configuration: .init())
        await doc.load()
        let (coordinator, tableView, _) = GridTestSupport.makeGrid(doc)

        withExtendedLifetime(coordinator) {
            tableView.moveColumn(0, toColumn: 2)
        }

        #expect(doc.table?.columns.map(\.cells) == [[ "b", "2" ], [ "c", "3" ],
                                                    [ "a", "1" ]])
    }

    @Test func columnMoveNotificationReordersTheDocument() async throws {
        let doc = CSVDocument(data: Data("a,b,c\n1,2,3\n".utf8), configuration: .init())
        await doc.load()
        let (coordinator, tableView, _) = GridTestSupport.makeGrid(doc)

        withExtendedLifetime(coordinator) {
            NotificationCenter.default.post(
                name: NSTableView.columnDidMoveNotification, object: tableView,
                userInfo: ["NSOldColumn": 0, "NSNewColumn": 2]
            )
        }

        #expect(doc.table?.columns.map(\.cells) == [[ "b", "2" ], [ "c", "3" ],
                                                    [ "a", "1" ]])
    }

    @Test func deletingASelectedColumnClearsTheHighlight() async throws {
        let doc = CSVDocument(data: Data("a,b\nc,d\n".utf8), configuration: .init())
        await doc.load()
        doc.selection = GridSelection(columns: IndexSet([1]))
        let (coordinator, tableView, headerView) = GridTestSupport.makeGrid(doc)
        #expect(headerView.selectedColumns == IndexSet([1]))

        doc.deleteSelection()
        GridTestSupport.update(coordinator, from: doc)

        #expect(doc.table?.columns.count == 1)
        #expect(doc.selection == GridSelection())
        #expect(headerView.selectedColumns.isEmpty)
        #expect(tableView.numberOfColumns == 1)
    }

    @Test func columnResizeSurvivesReorder() async throws {
        let doc = CSVDocument(data: Data("a,b,c\n1,2,3\n".utf8), configuration: .init())
        await doc.load()
        let (coordinator, tableView, _) = GridTestSupport.makeGrid(doc)
        tableView.tableColumns[1].width = 220

        withExtendedLifetime(coordinator) {
            tableView.moveColumn(0, toColumn: 2)
            GridTestSupport.update(coordinator, from: doc)
        }

        #expect(tableView.tableColumns[0].width == 220)
        #expect(tableView.tableColumns[1].width == 100)
    }

    @Test func columnMoveKeepsTheSelectedColumn() async throws {
        let doc = CSVDocument(data: Data("a,b,c\n1,2,3\n".utf8), configuration: .init())
        await doc.load()
        let (coordinator, tableView, headerView) = GridTestSupport.makeGrid(doc)
        withExtendedLifetime(coordinator) {
            headerView.clickColumn(at: 0, modifiers: [])
            NotificationCenter.default.post(
                name: NSTableView.columnDidMoveNotification, object: tableView,
                userInfo: ["NSOldColumn": 0, "NSNewColumn": 2]
            )
        }

        #expect(doc.selection == GridSelection(columns: IndexSet([2])))
    }

    /// Regression: equal cell values must not hide a column move.
    ///
    /// A new document is 3×1 with three identical empty columns. Content-only
    /// equality used to treat the move as a no-op, leaving the AppKit view and
    /// the model in different orders.
    @Test func movingAnIdenticalColumnUpdatesModelAndUndo() async throws {
        let um = UndoManager()
        let doc = CSVDocument()
        await doc.load()
        doc.setUndoManager(um)
        let (coordinator, tableView, _) = GridTestSupport.makeGrid(doc)
        let originalIDs = try #require(doc.table?.columns.map(\.id))
        #expect(Set(originalIDs).count == 3)

        withExtendedLifetime(coordinator) {
            tableView.moveColumn(0, toColumn: 2)
            GridTestSupport.update(coordinator, from: doc)
        }

        let movedIDs = try #require(doc.table?.columns.map(\.id))
        #expect(movedIDs == [originalIDs[1], originalIDs[2], originalIDs[0]])
        #expect(um.undoActionName == "Move Column")

        // The first visual column is the dragged column; an edit must land
        // under it, not under the column that used to be first.
        doc.setCell("typed", row: 0, column: 0)
        #expect(doc.table?.columns[0].id == originalIDs[1])
        #expect(doc.table?.cell(row: 0, column: 0) == "typed")

        um.undo() // cell edit
        um.undo() // column move
        #expect(doc.table?.columns.map(\.id) == originalIDs)
    }

    @Test func columnMoveIsAppliedOnceAcrossUpdates() async throws {
        let doc = CSVDocument(data: Data("a,b,c\n1,2,3\n".utf8), configuration: .init())
        await doc.load()
        let (coordinator, tableView, _) = GridTestSupport.makeGrid(doc)

        withExtendedLifetime(coordinator) {
            NotificationCenter.default.post(
                name: NSTableView.columnDidMoveNotification, object: tableView,
                userInfo: ["NSOldColumn": 0, "NSNewColumn": 2]
            )
            GridTestSupport.update(coordinator, from: doc)
            GridTestSupport.update(coordinator, from: doc)
        }

        #expect(doc.table?.columns.map(\.cells) == [[ "b", "2" ], [ "c", "3" ],
                                                    [ "a", "1" ]])
    }
}
