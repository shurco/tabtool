import Foundation
import Testing
@testable import TabToolKit

@Test func shiftedMovesEachSelectedIndexAndKeepsGaps() {
    #expect(CSVReorder.shifted(count: 5, moving: [1, 3], by: -1) == [1, 0, 3, 2, 4])
    #expect(CSVReorder.shifted(count: 5, moving: [1, 2], by: 1) == [0, 3, 1, 2, 4])
}

@Test func shiftedRejectsEdgesEmptyAndOutOfRange() {
    #expect(CSVReorder.shifted(count: 3, moving: [0, 2], by: -1) == nil)
    #expect(CSVReorder.shifted(count: 3, moving: [2], by: 1) == nil)
    #expect(CSVReorder.shifted(count: 3, moving: [], by: 1) == nil)
    #expect(CSVReorder.shifted(count: 3, moving: [5], by: -1) == nil)
    #expect(CSVReorder.shifted(count: 3, moving: [5], by: -3) == nil)
}

@Test func reorderRowsAppliesAPermutationToEveryColumn() {
    var t = CSVTable(rows: [["0a", "0b"], ["1a", "1b"], ["2a", "2b"]], headers: ["A", "B"])
    t.reorderRows([2, 0, 1])
    #expect(t.columns[0].cells == ["2a", "0a", "1a"])
    #expect(t.columns[1].cells == ["2b", "0b", "1b"])
    #expect(t.headers == ["A", "B"])
}

@Test func reorderIgnoresNonPermutations() {
    var t = CSVTable(rows: [["0"], ["1"]], headers: nil)
    t.reorderRows([0]); t.reorderRows([1, 1]); t.reorderRows([0, 2])
    #expect(t.columns[0].cells == ["0", "1"])
    var c = CSVTable(rows: [["a", "b", "c"]], headers: nil)
    let ids = c.columns.map(\.id)
    c.reorderColumns([2, 0, 1])
    #expect(c.columns.map(\.id) == [ids[2], ids[0], ids[1]])
    c.reorderColumns([0, 0, 1])
    #expect(c.columns.map(\.id) == [ids[2], ids[0], ids[1]])
}
