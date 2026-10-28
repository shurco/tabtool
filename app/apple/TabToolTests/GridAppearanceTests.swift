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
}
