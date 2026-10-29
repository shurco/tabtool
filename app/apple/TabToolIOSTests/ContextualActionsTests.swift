import Foundation
import Testing
@testable import TabTool

@Suite struct ContextualActionsTests {
    @Test func rowsOfferRowActions() {
        #expect(ContextualActions.ids(for: GridSelection(rows: [1]), allowsStructuralEdits: true)
            == [.addRowAbove, .addRowBelow, .delete])
    }

    @Test func columnsOfferColumnActions() {
        #expect(ContextualActions.ids(for: GridSelection(columns: [0]), allowsStructuralEdits: true)
            == [.addColumnLeft, .addColumnRight, .delete,
                .sortAscending, .sortDescending, .removeSort, .renameColumn])
    }

    @Test func nothingSelectedAndLargeModeShowNothing() {
        #expect(ContextualActions.ids(for: GridSelection(), allowsStructuralEdits: true) == [])
        #expect(ContextualActions.ids(for: GridSelection(rows: [1]), allowsStructuralEdits: false) == [])
    }
}
