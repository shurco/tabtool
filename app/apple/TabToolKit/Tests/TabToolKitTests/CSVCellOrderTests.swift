import Foundation
import Testing
@testable import TabToolKit

private func order(_ cells: [String], _ d: CSVSortDirection, mark: String = ".") -> [String] {
    let t = CSVTable(columns: [CSVColumn(header: nil, cells: cells)])
    return t.rowOrder(sortedByColumn: 0, direction: d, decimalMark: mark).map { cells[$0] }
}

@Test func ascendingPutsNumbersFirstThenNaturalTextThenEmpty() {
    #expect(order(["b", "", "10", "item10", "2", " ", "item2", "-3"], .ascending)
            == ["-3", "2", "10", "b", "item2", "item10", "", " "])
}

@Test func surroundingWhitespaceIsIgnoredWhenComparingText() {
    #expect(order([" b", "a", "c"], .ascending) == ["a", " b", "c"])
}

@Test func descendingReversesValuesButKeepsEmptyLast() {
    #expect(order(["1", "", "a", "3"], .descending) == ["a", "3", "1", ""])
}

@Test func decimalCommaIsNumericOnlyForCommaMark() {
    #expect(order(["1,5", "1,25", "1.0"], .ascending, mark: ",") == ["1,25", "1,5", "1.0"])
}

@Test func equalKeysKeepRowOrder() {
    let t = CSVTable(columns: [CSVColumn(header: nil, cells: ["x", "1", "x", "1"])])
    #expect(t.rowOrder(sortedByColumn: 0, direction: .ascending, decimalMark: ".") == [1, 3, 0, 2])
    #expect(t.rowOrder(sortedByColumn: 0, direction: .descending, decimalMark: ".") == [0, 2, 1, 3])
}

@Test func invalidColumnReturnsIdentity() {
    let t = CSVTable(rows: [["b"], ["a"]], headers: nil)
    #expect(t.rowOrder(sortedByColumn: 4, direction: .ascending, decimalMark: ".") == [0, 1])
}
