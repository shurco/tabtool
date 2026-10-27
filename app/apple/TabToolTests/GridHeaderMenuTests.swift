import AppKit
import Foundation
import Testing
import TabToolKit
@testable import TabTool

@Suite @MainActor struct GridHeaderMenuTests {
    private func loaded(_ csv: String) async -> CSVDocument {
        let doc = CSVDocument(data: Data(csv.utf8), configuration: .init(),
                              undoManager: UndoManager())
        await doc.load()
        return doc
    }

    private func perform(_ item: NSMenuItem) {
        _ = NSApp.sendAction(item.action!, to: item.target, from: item)
    }

    @Test func headerMenuListsSortAndMoveItems() async {
        let doc = await loaded("b,1\na,2\n")
        let (coordinator, _, _) = GridTestSupport.makeGrid(doc)
        let menu = coordinator.headerMenu(forColumn: 0)
        #expect(menu.items.map(\.title) == ["Sort Ascending", "Sort Descending", "Remove Sort",
                                            "", "Add Column Left", "Add Column Right",
                                            "", "Move Column Left", "Move Column Right",
                                            "", "Rename Column"])
        #expect(!menu.items[2].isEnabled)
    }

    @Test func renameItemIsEnabledOnlyWithHeaderMode() async {
        let plain = await loaded("a,b\n1,2\n")
        let (plainGrid, _, _) = GridTestSupport.makeGrid(plain)
        #expect(!plainGrid.headerMenu(forColumn: 0).items.last!.isEnabled)

        let doc = CSVDocument(data: Data("a,b\n1,2\n".utf8),
                              configuration: CSVConfiguration(firstRowAsHeader: true),
                              undoManager: UndoManager())
        await doc.load()
        let (coordinator, _, _) = GridTestSupport.makeGrid(doc)
        #expect(coordinator.headerMenu(forColumn: 0).items.last!.isEnabled)
    }

    @Test func renameEditorCommitsOnEndAndCancelsOnEscape() async {
        let doc = CSVDocument(data: Data("a,b\n1,2\n".utf8),
                              configuration: CSVConfiguration(firstRowAsHeader: true),
                              undoManager: UndoManager())
        await doc.load()
        let (coordinator, tableView, header) = GridTestSupport.makeGrid(doc)
        defer { withExtendedLifetime(coordinator) {} }
        tableView.frame = NSRect(x: 0, y: 0, width: 400, height: 100)

        coordinator.beginRenamingColumn(at: 1)
        let field = header.subviews.compactMap { $0 as? NSTextField }.first
        #expect(field?.stringValue == "b")
        field?.stringValue = "x"
        header.controlTextDidEndEditing(Notification(name: NSControl.textDidEndEditingNotification))
        #expect(doc.table?.headers == ["a", "x"])
        #expect(header.subviews.compactMap { $0 as? NSTextField }.isEmpty)

        GridTestSupport.update(coordinator, from: doc)
        coordinator.beginRenamingColumn(at: 0)
        _ = header.control(NSTextField(), textView: NSTextView(),
                           doCommandBy: #selector(NSResponder.cancelOperation(_:)))
        header.subviews.compactMap { $0 as? NSTextField }.first?.stringValue = "zzz"
        header.controlTextDidEndEditing(Notification(name: NSControl.textDidEndEditingNotification))
        #expect(doc.table?.headers == ["a", "x"])
    }

    @Test func doubleClickOnTitleRequestsRename() async {
        let doc = await loaded("a,b\n1,2\n")
        let (coordinator, tableView, header) = GridTestSupport.makeGrid(doc)
        defer { withExtendedLifetime(coordinator) {} }
        tableView.frame = NSRect(x: 0, y: 0, width: 400, height: 100)
        var requested: [Int] = []
        header.onRenameRequest = { requested.append($0) }

        let rect = header.headerRect(ofColumn: 0)
        func doubleClick(atX x: CGFloat) {
            let point = NSPoint(x: x, y: rect.midY)
            header.mouseDown(with: NSEvent.mouseEvent(
                with: .leftMouseDown, location: header.convert(point, to: nil),
                modifierFlags: [], timestamp: 0, windowNumber: 0, context: nil,
                eventNumber: 0, clickCount: 2, pressure: 1)!)
        }
        // Arrow and divider double-clicks fall through to AppKit's blocking
        // drag loop, so only the title hit is exercised here.
        doubleClick(atX: rect.minX + 10)
        #expect(requested == [0])
    }

    @Test func headerMenuAddColumnItemsInsertAroundTheColumn() async {
        let doc = await loaded("a,b\n1,2\n")
        let (coordinator, _, header) = GridTestSupport.makeGrid(doc)
        defer { withExtendedLifetime(coordinator) {} }
        perform(header.contextMenu(forColumn: 1)!.items[5])
        #expect(doc.table?.columns.count == 3)
        #expect(doc.table?.columns[1].cells == ["b", "2"])
        #expect(doc.table?.columns[2].cells == ["", ""])

        GridTestSupport.update(coordinator, from: doc)
        perform(header.contextMenu(forColumn: 0)!.items[4])
        #expect(doc.table?.columns.count == 4)
        #expect(doc.table?.columns[0].cells == ["", ""])
        #expect(doc.table?.columns[1].cells == ["a", "1"])
    }

    @Test func insertItemsAreDisabledWhenNotEditable() async {
        let doc = await loaded("a,b\n1,2\n")
        let (coordinator, _, _) = GridTestSupport.makeGrid(doc)
        coordinator.update(
            presenter: GridPresenter(table: doc.table ?? .newDocument(),
                                     configuration: doc.configuration),
            isEditable: false)
        #expect(!coordinator.headerMenu(forColumn: 0).items[4].isEnabled)
        #expect(!coordinator.rowMenu().items[0].isEnabled)
        #expect(!coordinator.rowMenu().items[1].isEnabled)
    }

    @Test func rowMenuInsertsAboveAndBelowTheRightClickedRow() async {
        let doc = await loaded("a\nb\nc\n")
        let (coordinator, tableView, _) = GridTestSupport.makeGrid(doc)
        defer { withExtendedLifetime(coordinator) {} }
        let menu = tableView.contextMenu(forRow: 1)!
        #expect(menu.items.map(\.title) == ["Add Row Above", "Add Row Below"])
        #expect(doc.selection.rows == [1])

        perform(menu.items[1])
        #expect(doc.table?.columns[0].cells == ["a", "b", "", "c"])
        GridTestSupport.update(coordinator, from: doc)
        perform(tableView.contextMenu(forRow: 0)!.items[0])
        #expect(doc.table?.columns[0].cells == ["", "a", "b", "", "c"])
    }

    @Test func rowContextMenuKeepsAnExistingSelection() async {
        let doc = await loaded("a\nb\nc\n")
        let (coordinator, tableView, _) = GridTestSupport.makeGrid(doc)
        defer { withExtendedLifetime(coordinator) {} }
        doc.selection = GridSelection(rows: [0, 2])
        GridTestSupport.update(coordinator, from: doc)
        _ = tableView.contextMenu(forRow: 2)
        #expect(doc.selection.rows == [0, 2])
        #expect(tableView.contextMenu(forRow: -1) == nil)
    }

    @Test func headerMenuSortAscendingSortsAndShowsArrow() async {
        let doc = await loaded("b,1\na,2\n")
        let (coordinator, _, header) = GridTestSupport.makeGrid(doc)
        perform(coordinator.headerMenu(forColumn: 0).items[0])
        GridTestSupport.update(coordinator, from: doc)

        #expect(doc.table?.columns[0].cells == ["a", "b"])
        #expect(header.sort?.column == 0)
        #expect(header.sort?.direction == .ascending)
    }

    @Test func arrowClearsWhenSortedColumnIsDeleted() async {
        let doc = await loaded("b,1\na,2\n")
        let (coordinator, _, header) = GridTestSupport.makeGrid(doc)
        doc.sortRows(column: 0, direction: .ascending)
        GridTestSupport.update(coordinator, from: doc)
        doc.deleteColumns(at: [0])
        GridTestSupport.update(coordinator, from: doc)

        #expect(header.sort == nil)
    }

    @Test func contextMenuSelectsTheColumnUnderTheMenu() async {
        let doc = await loaded("a,b\n1,2\n")
        let (coordinator, _, header) = GridTestSupport.makeGrid(doc)
        defer { withExtendedLifetime(coordinator) {} }
        #expect(header.contextMenu(forColumn: 1) != nil)
        #expect(doc.selection.columns == [1])

        doc.selection = GridSelection(columns: [0, 1])
        GridTestSupport.update(coordinator, from: doc)
        _ = header.contextMenu(forColumn: 0)
        #expect(doc.selection.columns == [0, 1])

        #expect(header.contextMenu(forColumn: -1) == nil)
    }

    @Test func arrowFollowsAMovedSortedColumn() async {
        let doc = await loaded("b,1\na,2\n")
        let (coordinator, _, header) = GridTestSupport.makeGrid(doc)
        doc.sortRows(column: 0, direction: .ascending)
        doc.selection = GridSelection(columns: [0])
        doc.moveSelectedColumns(by: 1)
        GridTestSupport.update(coordinator, from: doc)

        #expect(header.sort?.column == 1)
        #expect(header.sort?.direction == .ascending)
    }

    @Test func sortArrowClickCyclesWithoutSelecting() async {
        let doc = await loaded("b,1\na,2\n")
        let (coordinator, tableView, header) = GridTestSupport.makeGrid(doc)
        defer { withExtendedLifetime(coordinator) {} }
        tableView.frame = NSRect(x: 0, y: 0, width: 400, height: 100)
        doc.selection = GridSelection(rows: [0])
        GridTestSupport.update(coordinator, from: doc)

        let rect = header.headerRect(ofColumn: 0)
        let arrow = NSPoint(x: rect.maxX - 10, y: rect.midY)
        #expect(header.sortArrowColumn(at: arrow) == 0)
        #expect(header.sortArrowColumn(at: NSPoint(x: rect.maxX - 2, y: rect.midY)) == nil)
        #expect(header.sortArrowColumn(at: NSPoint(x: rect.minX + 10, y: rect.midY)) == nil)

        let click = NSEvent.mouseEvent(
            with: .leftMouseDown, location: header.convert(arrow, to: nil), modifierFlags: [],
            timestamp: 0, windowNumber: 0, context: nil, eventNumber: 0, clickCount: 1,
            pressure: 1)!
        header.mouseDown(with: click)
        #expect(doc.sortDirection(forColumn: 0) == .ascending)
        #expect(doc.selection.columns == [0])
    }

    @Test func sortClickDoesNothingWhenNotEditable() async {
        let doc = await loaded("b,1\na,2\n")
        let (coordinator, _, header) = GridTestSupport.makeGrid(doc)
        coordinator.update(
            presenter: GridPresenter(table: doc.table ?? .newDocument(),
                                     configuration: doc.configuration),
            isEditable: false)
        header.onSortClick?(0)
        #expect(doc.activeSort == nil)
    }
}
