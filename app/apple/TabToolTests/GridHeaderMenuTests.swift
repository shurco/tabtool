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
                                            "", "Move Column Left", "Move Column Right"])
        #expect(!menu.items[2].isEnabled)
    }

    @Test func headerMenuSortAscendingSortsAndShowsIndicator() async {
        let doc = await loaded("b,1\na,2\n")
        let (coordinator, tableView, _) = GridTestSupport.makeGrid(doc)
        perform(coordinator.headerMenu(forColumn: 0).items[0])
        GridTestSupport.update(coordinator, from: doc)

        #expect(doc.table?.columns[0].cells == ["a", "b"])
        let image = tableView.indicatorImage(in: tableView.tableColumns[0])
        #expect(image?.name() == "NSAscendingSortIndicator")
        #expect(tableView.indicatorImage(in: tableView.tableColumns[1]) == nil)
    }

    @Test func indicatorClearsWhenSortedColumnIsDeleted() async {
        let doc = await loaded("b,1\na,2\n")
        let (coordinator, tableView, _) = GridTestSupport.makeGrid(doc)
        doc.sortRows(column: 0, direction: .ascending)
        GridTestSupport.update(coordinator, from: doc)
        doc.deleteColumns(at: [0])
        GridTestSupport.update(coordinator, from: doc)

        for column in tableView.tableColumns {
            #expect(tableView.indicatorImage(in: column) == nil)
        }
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

    @Test func indicatorFollowsAMovedSortedColumn() async {
        let doc = await loaded("b,1\na,2\n")
        let (coordinator, tableView, _) = GridTestSupport.makeGrid(doc)
        doc.sortRows(column: 0, direction: .ascending)
        doc.selection = GridSelection(columns: [0])
        doc.moveSelectedColumns(by: 1)
        GridTestSupport.update(coordinator, from: doc)

        #expect(tableView.indicatorImage(in: tableView.tableColumns[0]) == nil)
        #expect(tableView.indicatorImage(in: tableView.tableColumns[1])?.name()
                == "NSAscendingSortIndicator")
    }
}
