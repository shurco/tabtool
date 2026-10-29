import CoreGraphics
import Testing
import TabToolKit
@testable import TabTool

@Suite struct GridGeometryTests {
    @Test func firstCellSitsBelowTheHeaderAndBesideTheGutter() {
        let geometry = GridGeometry(rowHeight: 24, showsRowNumbers: true, rowCount: 2, columnCount: 3)
        #expect(geometry.cellFrame(row: 0, column: 0)
            == CGRect(x: 48, y: 28, width: 120, height: 24))
        #expect(geometry.contentSize == CGSize(width: 48 + 360, height: 28 + 48))
    }

    @Test func hidingRowNumbersRemovesTheGutter() {
        let geometry = GridGeometry(rowHeight: 24, showsRowNumbers: false, rowCount: 1, columnCount: 1)
        #expect(geometry.cellFrame(row: 0, column: 0).origin.x == 0)
        #expect(geometry.contentSize.width == 120)
    }

    @Test func pinnedHeaderAndGutterFollowTheVisibleRect() {
        let geometry = GridGeometry(rowHeight: 24, showsRowNumbers: true, rowCount: 20, columnCount: 8)
        let visible = CGRect(x: 200, y: 100, width: 300, height: 400)
        #expect(geometry.pinnedHeaderFrame(column: 1, visible: visible).origin.y == 100)
        #expect(geometry.pinnedHeaderFrame(column: 1, visible: visible).origin.x
            == GridGeometry.gutterWidth + GridGeometry.columnWidth)
        #expect(geometry.pinnedGutterFrame(row: 3, visible: visible).origin.x == 200)
        #expect(geometry.pinnedCornerFrame(visible: visible).origin == CGPoint(x: 200, y: 100))
    }
}

@Suite struct GridRowMappingTests {
    @Test func filteredRowsKeepModelNumbers() {
        let table = CSVTable(rows: [["a"], ["b"], ["c"]], headers: nil)
        let presenter = GridPresenter(table: table, configuration: .init(), visibleRows: [0, 2])
        #expect(presenter.numberOfRows == 2)
        #expect(presenter.modelRow(forDisplayRow: 0) == 0)
        #expect(presenter.modelRow(forDisplayRow: 1) == 2)
        #expect(presenter.cell(row: 1, column: 0) == "c")
    }
}
