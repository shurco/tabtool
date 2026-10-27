import Foundation
import Testing
@testable import TabToolKit

@Test func raggedRowsArePaddedNotTruncated() {
    let t = CSVTable(rows: [["a","b","c"], ["d"]], headers: ["h1","h2","h3"])
    #expect(t.rowCount == 2)
    #expect(t.columns.count == 3)
    #expect(t.cell(row: 1, column: 2) == "")
    #expect(t.headers == ["h1","h2","h3"])
}

@Test func newDocumentIsThreeByOne() {
    let t = CSVTable.newDocument()
    #expect(t.rowCount == 1)
    #expect(t.columns.count == 3)
    #expect(t.headers == nil)
}

@Test func emptyParsedRowBecomesOneByOne() {
    let t = CSVTable(rows: [[""]], headers: nil)
    #expect(t.rowCount == 1)
    #expect(t.columns.count == 1)
    #expect(t.cell(row: 0, column: 0) == "")
}

@Test func addDeleteAndMoveRowsKeepAllColumnsAligned() {
    var t = CSVTable(rows: [["0a","0b"],["1a","1b"],["2a","2b"]], headers: nil)
    t.addRow(at: 1)
    #expect(t.rowCount == 4)
    #expect(t.columns[0].cells == ["0a","","1a","2a"])
    #expect(t.columns[1].cells == ["0b","","1b","2b"])
    t.deleteRows(at: IndexSet([1]))
    #expect(t.columns[0].cells == ["0a","1a","2a"])
    t.moveRows(at: IndexSet([0]), to: 3)
    #expect(t.columns[0].cells == ["1a","2a","0a"])
}

@Test func addDeleteAndMoveColumnsPreserveHeaders() {
    var t = CSVTable(rows: [["a","b"],["c","d"]], headers: ["A","B"])
    t.addColumn(at: 0, header: "X")
    #expect(t.headers == ["X","A","B"])
    #expect(t.columns.map(\.cells) == [["",""],["a","c"],["b","d"]])
    t.moveColumn(from: 2, to: 0)
    #expect(t.headers == ["B","X","A"])
    t.deleteColumns(at: IndexSet([1]))
    #expect(t.headers == ["B","A"])
}

@Test func moveNonContiguousRowsToEndKeepsOrder() {
    var t = CSVTable(rows: [["0"],["1"],["2"],["3"],["4"]], headers: nil)
    t.moveRows(at: IndexSet([1,3]), to: 5)
    #expect(t.columns[0].cells == ["0","2","4","1","3"])
}

@Test func headersAreAllOrNothing() {
    var t = CSVTable(rows: [["a","b"]], headers: ["A","B"])
    t.setHeaders(nil)
    #expect(t.headers == nil)
    #expect(t.columns.allSatisfy { $0.header == nil })
    t.setHeaders(["only-one"])
    #expect(t.headers == ["only-one", ""])
}

@Test func decimalConversionOnlyTouchesNumericCells() {
    var t = CSVTable(rows: [["1,50"], ["a,b"], ["001,00"]], headers: nil)
    t.convertDecimalMark(from: ",", to: ".")
    #expect(t.columns[0].cells == ["1.50","a,b","001.00"])
}

@Test func initializingColumnsPadsToLongest() {
    let t = CSVTable(columns: [
        CSVColumn(header: "A", cells: ["1"]),
        CSVColumn(header: "B", cells: ["2", "3", "4"])
    ])
    #expect(t.rowCount == 3)
    #expect(t.columns.map(\.cells) == [["1", "", ""], ["2", "3", "4"]])
    #expect(t.headers == ["A", "B"])
}

@Test func mixedColumnHeadersCollapseToNoHeaders() {
    let t = CSVTable(columns: [
        CSVColumn(header: "A", cells: ["1"]),
        CSVColumn(header: nil, cells: ["2"])
    ])
    #expect(t.headers == nil)
    #expect(t.columns.allSatisfy { $0.header == nil })
}

@Test func headersLongerThanRowsExtendColumns() {
    let t = CSVTable(rows: [["a"]], headers: ["h1", "h2"])
    #expect(t.columns.count == 2)
    #expect(t.columns.map(\.cells) == [["a"], [""]])
    #expect(t.headers == ["h1", "h2"])
}

@Test func invalidIndexesAreIgnored() {
    var t = CSVTable(rows: [["a"]], headers: nil)
    #expect(t.cell(row: -1, column: 0) == "")
    #expect(t.cell(row: 0, column: 5) == "")
    t.setCell("x", row: 9, column: 0)
    t.setCell("x", row: 0, column: 9)
    #expect(t.cell(row: 0, column: 0) == "a")
    t.deleteRows(at: IndexSet([5, 10]))
    t.deleteColumns(at: IndexSet([9]))
    t.moveRows(at: IndexSet([7]), to: 0)
    t.moveColumn(from: 4, to: 0)
    #expect(t.rowCount == 1)
    #expect(t.columns.count == 1)
}

@Test func outOfRangeInsertionsClamp() {
    var t = CSVTable(rows: [["a"]], headers: nil)
    t.addRow(at: 99)
    #expect(t.columns[0].cells == ["a", ""])
    t.addRow(at: -3)
    #expect(t.columns[0].cells == ["", "a", ""])
    t.addColumn(at: 99)
    #expect(t.columns.map(\.cells) == [["", "a", ""], ["", "", ""]])
}

@Test func tableEqualityComparesContentNotColumnIdentity() {
    let first = CSVTable(rows: [["a", "b"]], headers: ["A", "B"])
    let second = CSVTable(rows: [["a", "b"]], headers: ["A", "B"])
    #expect(first == second)

    var changed = second
    changed.setCell("x", row: 0, column: 1)
    #expect(first != changed)
    changed.setHeaders(nil)
    #expect(first != changed)
}

@Test func columnOrderComparisonDetectsIdentityEvenWhenContentMatches() {
    var t = CSVTable(rows: [["", "", ""]], headers: nil)
    let original = t
    t.moveColumn(from: 0, to: 2)

    // Content equality alone cannot see the move: all cells are empty.
    #expect(t == original)
    #expect(!t.hasSameColumnOrder(as: original))
    #expect(t.hasSameColumnOrder(as: t))
}

@Test func addColumnOnHeaderlessTableIgnoresHeader() {
    var t = CSVTable(rows: [["a"]], headers: nil)
    t.addColumn(at: 1, header: "X")
    #expect(t.headers == nil)
    #expect(t.columns.map(\.header) == [nil, nil])
    #expect(t.columns.map(\.cells) == [["a"], [""]])
}

@Test func renameHeaderChangesOnlyThatColumnAndKeepsIdentity() {
    var t = CSVTable(rows: [["1", "2"]], headers: ["a", "b"])
    let ids = t.columns.map(\.id)
    let tRenamed994 = t.renameHeader("x", at: 1)
    #expect(tRenamed994)
    #expect(t.headers == ["a", "x"])
    #expect(t.columns.map(\.id) == ids)
    #expect(t.columns.map(\.cells) == [["1"], ["2"]])
}

@Test func renameHeaderAllowsEmptyName() {
    var t = CSVTable(rows: [["1"]], headers: ["a"])
    let tRenamed743 = t.renameHeader("", at: 0)
    #expect(tRenamed743)
    #expect(t.headers == [""])
}

@Test func renameHeaderWithoutHeadersGeneratesNamesAndKeepsAllRows() {
    var t = CSVTable(rows: [["1", "2"], ["3", "4"]], headers: nil)
    let r1 = t.renameHeader("x", at: 1)
    #expect(r1)
    #expect(t.headers == ["A", "x"])
    #expect(t.rowCount == 2)
    #expect(t.columns.map(\.cells) == [["1", "3"], ["2", "4"]])
}

@Test func renameHeaderToGeneratedNameWithoutHeadersChangesNothing() {
    var t = CSVTable(rows: [["1", "2"]], headers: nil)
    let r2 = t.renameHeader("B", at: 1)
    #expect(!r2)
    #expect(t.headers == nil)
}

@Test func renameHeaderIsRejectedForBadIndex() {
    var plain = CSVTable(rows: [["1"]], headers: nil)
    let r3 = plain.renameHeader("x", at: 3)
    #expect(!r3)
    #expect(plain.headers == nil)

    var named = CSVTable(rows: [["1"]], headers: ["a"])
    let r4 = named.renameHeader("x", at: 3)
    #expect(!r4)
    #expect(named.headers == ["a"])
}
