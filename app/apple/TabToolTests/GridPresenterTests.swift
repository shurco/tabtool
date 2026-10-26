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
