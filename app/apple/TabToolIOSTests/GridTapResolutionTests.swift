@testable import TabTool
import Testing

@Suite struct GridTapResolutionTests {
    private let cell = GridFocus(displayRow: 2, column: 3)

    @Test func firstTapSelects() {
        #expect(GridTapResolution.resolve(element: .cell, focus: nil, tapped: cell) == .selectCell)
    }

    @Test func secondTapOnTheFocusedCellEdits() {
        #expect(GridTapResolution.resolve(element: .cell, focus: cell, tapped: cell) == .editCell)
    }

    @Test func tapOnAnotherCellSelectsInsteadOfEditing() {
        let other = GridFocus(displayRow: 2, column: 4)
        #expect(GridTapResolution.resolve(element: .cell, focus: cell, tapped: other) == .selectCell)
    }

    @Test func headerAndRowNumberSelectTheirAxis() {
        #expect(GridTapResolution.resolve(element: .columnHeader, focus: cell, tapped: nil) == .selectColumn)
        #expect(GridTapResolution.resolve(element: .rowNumber, focus: cell, tapped: nil) == .selectRowNumber)
    }

    @Test func cornerAndEmptyHitsAreIgnored() {
        #expect(GridTapResolution.resolve(element: .corner, focus: cell, tapped: nil) == .ignore)
        #expect(GridTapResolution.resolve(element: .cell, focus: nil, tapped: nil) == .ignore)
    }
}
