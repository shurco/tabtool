import AppKit
import Testing
@testable import TabTool

@Suite @MainActor struct GridAppearanceTests {
    @Test func appearanceIsAppliedToTheTableView() {
        let table = NSTableView()
        table.applyGridAppearance(alternatingRows: false, rowHeight: 32)
        #expect(table.usesAlternatingRowBackgroundColors == false)
        #expect(table.rowHeight == 32)
    }

    @Test func rowsUseTheGridRowView() async {
        let document = await GridTestSupport.makeLoadedNewDocument()
        let (coordinator, tableView, _) = GridTestSupport.makeGrid(document)
        withExtendedLifetime(coordinator) {
            #expect(tableView.rowView(atRow: 0, makeIfNecessary: true) is CSVTableRowView)
        }
    }
}
