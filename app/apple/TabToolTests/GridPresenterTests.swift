import AppKit
import Testing
import TabToolKit
@testable import TabTool

@Suite struct GridPresenterTests {
    @Test func headersPreferDocumentHeaders() {
        let t = CSVTable(rows: [["a", "b"]], headers: ["h1", "h2"])
        let p = GridPresenter(table: t, configuration: .init())
        #expect(p.header(at: 0) == "h1")
        #expect(p.header(at: 1) == "h2")
    }

    @Test func headersFallBackToGeneratedNames() {
        let t = CSVTable(rows: [["a", "b"]], headers: nil)
        let p = GridPresenter(table: t, configuration: .init())
        #expect(p.header(at: 0) == "A")
        #expect(p.header(at: 1) == "B")
    }

    @Test func cellsAndAlignmentMatchConfiguration() {
        let t = CSVTable(rows: [["1,5", "x"]], headers: nil)
        let p = GridPresenter(table: t, configuration: CSVConfiguration(decimalMark: ","))
        #expect(p.cell(row: 0, column: 0) == "1,5")
        #expect(p.alignment(row: 0, column: 0) == .right)
        #expect(p.alignment(row: 0, column: 1) == .left)
    }

    @Test func rowAndColumnCountsFollowTable() {
        let t = CSVTable(rows: [["a", "b", "c"], ["d", "e", "f"]], headers: nil)
        let p = GridPresenter(table: t, configuration: .init())
        #expect(p.numberOfRows == 2)
        #expect(p.numberOfColumns == 3)
    }
}

private struct StubSource: RowSource {
    let rowCount = 3
    let columnCount = 2
    let columnIDs = [UUID(), UUID()]
    let headers: [String]? = nil

    func cell(row: Int, column: Int) -> String {
        row < rowCount && column < columnCount ? "\(row * 10 + column)" : ""
    }
}

@Suite struct GridPresenterRowSourceTests {
    @Test func presenterReadsAnyRowSource() {
        let p = GridPresenter(table: StubSource(), configuration: .init())
        #expect(p.numberOfRows == 3)
        #expect(p.numberOfColumns == 2)
        #expect(p.header(at: 1) == "B")
        #expect(p.header(at: 2) == "")
        #expect(p.cell(row: 2, column: 1) == "21")
        #expect(p.alignment(row: 2, column: 1) == .right)
        #expect(TableSignature(presenter: p).columnIDs.count == 2)
    }
}

@Suite struct GridPresenterFilteredRowsTests {
    private let table = CSVTable(rows: [["0", "x"], ["1", "y"], ["2", "z"], ["3", "w"]], headers: nil)

    private func presenter(visibleRows: [Int]?) -> GridPresenter {
        GridPresenter(table: table, configuration: .init(), visibleRows: visibleRows)
    }

    @Test func withoutVisibleRowsEveryRowMapsToItself() {
        let p = presenter(visibleRows: nil)
        #expect(p.numberOfRows == 4)
        #expect(p.modelRow(forDisplayRow: 3) == 3)
        #expect(p.displayRow(forModelRow: 3) == 3)
        #expect(p.cell(row: 2, column: 1) == "z")
    }

    @Test func visibleRowsDefineTheDisplayedRows() {
        let p = presenter(visibleRows: [1, 3])
        #expect(p.numberOfRows == 2)
        #expect(p.cell(row: 0, column: 1) == "y")
        #expect(p.cell(row: 1, column: 1) == "w")
        #expect(p.alignment(row: 1, column: 0) == .right)
        #expect(p.alignment(row: 1, column: 1) == .left)
    }

    @Test func rowsMapBetweenDisplayAndModelIndexes() {
        let p = presenter(visibleRows: [1, 3])
        #expect(p.modelRow(forDisplayRow: 1) == 3)
        #expect(p.displayRow(forModelRow: 3) == 1)
        #expect(p.displayRow(forModelRow: 2) == nil)
        #expect(p.modelRows(forDisplayRows: IndexSet([0, 1])) == IndexSet([1, 3]))
        #expect(p.displayRows(forModelRows: IndexSet([0, 1, 2, 3])) == IndexSet([0, 1]))
    }

    @Test func noVisibleRowShowsAnEmptyGrid() {
        let p = presenter(visibleRows: [])
        #expect(p.numberOfRows == 0)
        #expect(p.displayRows(forModelRows: IndexSet([0, 1])).isEmpty)
    }

    @Test func tableSignatureIncludesTheVisibleRows() {
        let all = TableSignature(presenter: presenter(visibleRows: nil))
        let some = TableSignature(presenter: presenter(visibleRows: [1, 3]))
        let other = TableSignature(presenter: presenter(visibleRows: [0, 2]))
        #expect(all != some)
        #expect(some != other)
        #expect(some == TableSignature(presenter: presenter(visibleRows: [1, 3])))
        #expect(some.rowCount == 2)
    }
}
