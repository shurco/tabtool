import Foundation
import Testing
@testable import TabToolKit

private let names = ["Alice", "bob", "Carol", "alice smith", "", "Dave"]
private let values = ["10", "9", "2.5", "abc", "", "10.0"]

private func makeTable() -> CSVTable {
    CSVTable(rows: zip(names, values).map { [$0, $1] }, headers: nil, minimumColumnCount: 2)
}

/// Rows of `makeTable()` matched by one criterion on `column`.
private func match(_ op: FilterOperator, _ value: String = "", column: Int = 0,
                   mark: String = ".", table: CSVTable = makeTable()) -> [Int] {
    let criterion = FilterCriterion(columnID: table.columns[column].id, op: op, value: value)
    return RowFilter(criteria: [criterion]).matchingRows(in: table, decimalMark: mark)
}

@Test func equalsIsCaseInsensitiveText() {
    #expect(match(.equals, "ALICE") == [0])
}

@Test func notEqualsKeepsEverythingElse() {
    #expect(match(.notEquals, "alice") == [1, 2, 3, 4, 5])
}

@Test func containsIsCaseInsensitive() {
    #expect(match(.contains, "ALIC") == [0, 3])
}

@Test func notContainsExcludesMatches() {
    #expect(match(.notContains, "alic") == [1, 2, 4, 5])
}

@Test func beginsWithAnchorsAtStart() {
    #expect(match(.beginsWith, "ALICE") == [0, 3])
    #expect(match(.beginsWith, "smith").isEmpty)
}

@Test func endsWithAnchorsAtEnd() {
    #expect(match(.endsWith, "SMITH") == [3])
    #expect(match(.endsWith, "alice") == [0])
}

@Test func numericEqualsComparesNumbers() {
    #expect(match(.equals, "10", column: 1) == [0, 5])
}

@Test func numericOrderingComparesNumbersNotText() {
    // Text order would put "10" before "9"; the non-numeric "abc" falls back to text.
    #expect(match(.greaterThan, "9", column: 1) == [0, 3, 5])
    #expect(match(.lessThan, "10", column: 1) == [1, 2])
    #expect(match(.greaterOrEqual, "9", column: 1) == [0, 1, 3, 5])
    #expect(match(.lessOrEqual, "9", column: 1) == [1, 2])
}

@Test func nonNumericSideFallsBackToCaseInsensitiveTextComparison() {
    #expect(match(.equals, "ABC", column: 1) == [3])
    #expect(match(.greaterThan, "ALICE Z", column: 0) == [1, 2, 5])
    #expect(match(.lessThan, "b", column: 0) == [0, 3])
}

@Test func emptyCellNeverSatisfiesEqualsOrOrdering() {
    #expect(!match(.equals, "x").contains(4))
    #expect(!match(.lessThan, "z").contains(4))
    #expect(!match(.lessOrEqual, "10", column: 1).contains(4))
    #expect(match(.notEquals, "x").contains(4))
}

@Test func decimalMarkControlsNumericParsing() {
    let table = CSVTable(rows: [["1,5"], ["1,50"], ["2,0"]], headers: nil)
    #expect(match(.equals, "1,5", mark: ",", table: table) == [0, 1])
    #expect(match(.greaterThan, "1,5", mark: ",", table: table) == [2])
}

@Test func isEmptyAndIsNotEmptyIgnoreTheValue() {
    #expect(match(.isEmpty, "ignored") == [4])
    #expect(match(.isNotEmpty) == [0, 1, 2, 3, 5])
}

@Test func whitespaceOnlyCellCountsAsEmpty() {
    let table = CSVTable(rows: [[" "], ["x"]], headers: nil)
    #expect(match(.isEmpty, table: table) == [0])
}

@Test func regexMatchesAnywhereAndIsCaseInsensitive() {
    #expect(match(.matchesRegex, "^a.*e$") == [0])
    #expect(match(.matchesRegex, "SMITH|dave") == [3, 5])
}

@Test func invalidRegexIsInvalidAndIgnored() {
    let criterion = FilterCriterion(columnID: makeTable().columns[0].id,
                                    op: .matchesRegex, value: "(")
    #expect(!criterion.isValid)
    #expect(match(.matchesRegex, "(").count == names.count)
}

@Test func validCriteriaReportValid() {
    #expect(FilterCriterion(op: .matchesRegex, value: "a+").isValid)
    #expect(FilterCriterion(op: .matchesRegex, value: "").isValid)
    #expect(FilterCriterion(op: .contains, value: "(").isValid)
}

@Test func emptyValueIsIgnoredForValueOperators() {
    for op in FilterOperator.allCases where op.needsValue {
        #expect(match(op).count == names.count)
    }
}

@Test func needsValueIsFalseOnlyForEmptinessOperators() {
    let withoutValue = FilterOperator.allCases.filter { !$0.needsValue }
    #expect(Set(withoutValue) == [.isEmpty, .isNotEmpty])
}

@Test func disabledCriterionIsIgnored() {
    let table = makeTable()
    let criterion = FilterCriterion(isEnabled: false, columnID: table.columns[0].id,
                                    op: .equals, value: "bob")
    #expect(RowFilter(criteria: [criterion]).matchingRows(in: table).count == names.count)
}

@Test func criteriaCombineWithAnd() {
    let table = makeTable()
    let filter = RowFilter(criteria: [
        FilterCriterion(columnID: table.columns[0].id, op: .contains, value: "alice"),
        FilterCriterion(columnID: table.columns[1].id, op: .lessThan, value: "10")
    ])
    #expect(filter.matchingRows(in: table) == [])
    let loose = RowFilter(criteria: [
        FilterCriterion(columnID: table.columns[0].id, op: .contains, value: "alice"),
        FilterCriterion(columnID: table.columns[1].id, op: .greaterOrEqual, value: "10")
    ])
    // "abc" is not numeric, so it compares as text and sorts after "10".
    #expect(loose.matchingRows(in: table) == [0, 3])
}

@Test func missingOrUnknownColumnIsIgnored() {
    let table = makeTable()
    let filter = RowFilter(criteria: [
        FilterCriterion(columnID: nil, op: .equals, value: "x"),
        FilterCriterion(columnID: UUID(), op: .equals, value: "x")
    ])
    #expect(filter.matchingRows(in: table).count == names.count)
}

@Test func noActiveCriteriaReturnsAllRowsInOrder() {
    #expect(RowFilter().matchingRows(in: makeTable()) == Array(0..<names.count))
}

@Test func emptyTableYieldsNoRows() {
    let table = CSVTable(rows: [], headers: nil)
    let criterion = FilterCriterion(columnID: table.columns[0].id, op: .isEmpty)
    #expect(RowFilter(criteria: [criterion]).matchingRows(in: table).isEmpty)
}

@Test func isActiveNeedsAtLeastOneUsableCriterion() {
    let table = makeTable()
    let id = table.columns[0].id
    func isActive(_ criteria: FilterCriterion...) -> Bool {
        RowFilter(criteria: criteria).isActive(in: table)
    }
    #expect(!RowFilter().isActive(in: table))
    #expect(!isActive(FilterCriterion(columnID: id, op: .contains, value: "")))
    #expect(!isActive(FilterCriterion(isEnabled: false, columnID: id, op: .contains, value: "a")))
    #expect(!isActive(FilterCriterion(columnID: nil, op: .contains, value: "a")))
    #expect(!isActive(FilterCriterion(columnID: UUID(), op: .contains, value: "a")))
    #expect(!isActive(FilterCriterion(columnID: id, op: .matchesRegex, value: "(")))
    #expect(isActive(FilterCriterion(columnID: id, op: .contains, value: "a")))
    #expect(isActive(FilterCriterion(columnID: id, op: .isEmpty)))
    #expect(isActive(FilterCriterion(columnID: id, op: .contains, value: ""),
                     FilterCriterion(columnID: id, op: .contains, value: "a")))
}
