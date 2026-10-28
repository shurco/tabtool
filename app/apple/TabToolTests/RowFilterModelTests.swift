import Foundation
import Testing
import TabToolKit
@testable import TabTool

@Suite @MainActor struct RowFilterModelTests {
    private let table = CSVTable(rows: [["a", "1"], ["b", "2"], ["a", "3"], ["c", "4"]],
                                 headers: ["name", "number"])

    private var nameID: UUID { table.columnIDs[0] }
    private var numberID: UUID { table.columnIDs[1] }

    private func model(_ criteria: FilterCriterion...) -> RowFilterModel {
        let model = RowFilterModel()
        model.criteria = criteria
        return model
    }

    @Test func startsWithOneEmptyCriterionAndNoFilter() {
        let model = RowFilterModel()
        #expect(model.criteria.count == 1)
        #expect(model.criteria[0].columnID == nil)
        #expect(model.criteria[0].value.isEmpty)
        #expect(model.appliedFilter == nil)
        #expect(model.visibleRows == nil)
        #expect(!model.isFiltering)
        #expect(!model.isBarVisible)
    }

    @Test func applyAllCombinesEveryCriterionWithAnd() {
        let model = model(
            FilterCriterion(columnID: nameID, op: .equals, value: "a"),
            FilterCriterion(columnID: numberID, op: .greaterThan, value: "1")
        )
        model.applyAll(to: table, decimalMark: ".")
        #expect(model.visibleRows == [2])
        #expect(model.isFiltering)
        #expect(model.appliedFilter == RowFilter(criteria: model.criteria))
    }

    @Test func applyCriterionUsesOnlyThatCriterionEvenWhenDisabled() {
        let first = FilterCriterion(isEnabled: false, columnID: nameID, op: .equals, value: "a")
        let second = FilterCriterion(columnID: numberID, op: .greaterThan, value: "1")
        let model = model(first, second)

        model.apply(criterionID: first.id, to: table, decimalMark: ".")
        #expect(model.visibleRows == [0, 2])
        #expect(model.appliedFilter?.criteria.count == 1)

        model.apply(criterionID: second.id, to: table, decimalMark: ".")
        #expect(model.visibleRows == [1, 2, 3])
    }

    @Test func applyUnknownCriterionChangesNothing() {
        let model = model(FilterCriterion(columnID: nameID, value: "a"))
        model.apply(criterionID: UUID(), to: table, decimalMark: ".")
        #expect(!model.isFiltering)
    }

    @Test func applyHonoursTheDecimalMark() {
        let source = CSVTable(rows: [["1,5"], ["2,5"]], headers: nil)
        let model = model(FilterCriterion(columnID: source.columnIDs[0], op: .greaterThan, value: "2,0"))
        model.applyAll(to: source, decimalMark: ",")
        #expect(model.visibleRows == [1])
    }

    @Test func clearRemovesTheFilterAndResetsTheDraft() {
        let model = model(
            FilterCriterion(columnID: nameID, value: "a"),
            FilterCriterion(columnID: numberID, value: "1")
        )
        model.applyAll(to: table, decimalMark: ".")
        model.clear()
        #expect(model.appliedFilter == nil)
        #expect(model.visibleRows == nil)
        #expect(model.criteria.count == 1)
        #expect(model.criteria[0].columnID == nil)
        #expect(model.criteria[0].value.isEmpty)
    }

    @Test func addCriterionInsertsAnEmptyRowAfterTheGivenOne() {
        let first = FilterCriterion(columnID: nameID, value: "a")
        let second = FilterCriterion(columnID: numberID, value: "1")
        let model = model(first, second)

        model.addCriterion(after: first.id)
        #expect(model.criteria.count == 3)
        #expect(model.criteria[0].id == first.id)
        #expect(model.criteria[1].columnID == nil)
        #expect(model.criteria[2].id == second.id)
    }

    @Test func addCriterionAfterAnUnknownIDAppends() {
        let model = model(FilterCriterion(columnID: nameID, value: "a"))
        model.addCriterion(after: UUID())
        #expect(model.criteria.count == 2)
        #expect(model.criteria[1].columnID == nil)
    }

    @Test func removeCriterionDropsItButAlwaysKeepsOneRow() {
        let first = FilterCriterion(columnID: nameID, value: "a")
        let second = FilterCriterion(columnID: numberID, value: "1")
        let model = model(first, second)

        model.removeCriterion(id: first.id)
        #expect(model.criteria.map(\.id) == [second.id])

        model.removeCriterion(id: second.id)
        #expect(model.criteria.count == 1)
        #expect(model.criteria[0].id != second.id)
        #expect(model.criteria[0].columnID == nil)
        #expect(model.criteria[0].value.isEmpty)
    }

    @Test func removeCriterionDoesNotTouchTheAppliedFilter() {
        let first = FilterCriterion(columnID: nameID, value: "a")
        let second = FilterCriterion(columnID: numberID, value: "1")
        let model = model(first, second)
        model.applyAll(to: table, decimalMark: ".")
        model.removeCriterion(id: first.id)
        #expect(model.appliedFilter?.criteria.count == 2)
    }

    @Test func refreshReevaluatesTheAppliedFilter() {
        let model = model(FilterCriterion(columnID: nameID, op: .equals, value: "a"))
        model.applyAll(to: table, decimalMark: ".")
        #expect(model.visibleRows == [0, 2])

        var edited = table
        edited.setCell("z", row: 0, column: 0)
        model.refresh(source: edited, decimalMark: ".")
        #expect(model.visibleRows == [2])
    }

    @Test func refreshWithoutAFilterKeepsEverythingVisible() {
        let model = model(FilterCriterion(columnID: nameID, value: "a"))
        model.refresh(source: table, decimalMark: ".")
        #expect(model.visibleRows == nil)
        #expect(!model.isFiltering)
    }

    @Test func refreshDropsAppliedCriteriaOfRemovedColumns() {
        let model = model(
            FilterCriterion(columnID: nameID, op: .equals, value: "a"),
            FilterCriterion(columnID: numberID, op: .greaterThan, value: "1")
        )
        model.applyAll(to: table, decimalMark: ".")

        var withoutNumber = table
        withoutNumber.deleteColumns(at: IndexSet(integer: 1))
        model.refresh(source: withoutNumber, decimalMark: ".")
        #expect(model.appliedFilter?.criteria.map(\.columnID) == [nameID])
        #expect(model.visibleRows == [0, 2])
    }

    @Test func refreshClearsTheFilterWhenNoCriterionKeepsItsColumn() {
        let model = model(FilterCriterion(columnID: numberID, op: .greaterThan, value: "1"))
        model.applyAll(to: table, decimalMark: ".")

        var withoutNumber = table
        withoutNumber.deleteColumns(at: IndexSet(integer: 1))
        model.refresh(source: withoutNumber, decimalMark: ".")
        #expect(!model.isFiltering)
        #expect(model.visibleRows == nil)
    }

    @Test func refreshDetachesDraftCriteriaFromRemovedColumns() {
        let model = model(
            FilterCriterion(columnID: nameID, value: "a"),
            FilterCriterion(columnID: numberID, value: "1")
        )
        var withoutNumber = table
        withoutNumber.deleteColumns(at: IndexSet(integer: 1))
        model.refresh(source: withoutNumber, decimalMark: ".")
        #expect(model.criteria.count == 2)
        #expect(model.criteria[0].columnID == nameID)
        #expect(model.criteria[1].columnID == nil)
    }

    @Test func refreshKeepsCriteriaOnTheSamePositionsAfterAReload() {
        let model = model(
            FilterCriterion(columnID: nameID, op: .equals, value: "a"),
            FilterCriterion(columnID: numberID, op: .greaterThan, value: "1")
        )
        model.applyAll(to: table, decimalMark: ".")

        let reloaded = CSVTable(rows: [["a", "0"], ["a", "5"], ["b", "9"]], headers: ["name", "number"])
        model.refresh(source: reloaded, decimalMark: ".")
        #expect(model.criteria.map(\.columnID) == reloaded.columnIDs)
        #expect(model.appliedFilter?.criteria.map(\.columnID) == reloaded.columnIDs.map { Optional($0) })
        #expect(model.visibleRows == [1])
    }

    @Test func refreshKeepsRemappedCriteriaOnTheNextEdit() {
        let model = model(FilterCriterion(columnID: nameID, op: .equals, value: "a"))
        model.applyAll(to: table, decimalMark: ".")
        let reloaded = CSVTable(rows: [["a", "0"]], headers: nil, minimumColumnCount: 2)
        model.refresh(source: reloaded, decimalMark: ".")

        var edited = reloaded
        edited.setCell("z", row: 0, column: 0)
        model.refresh(source: edited, decimalMark: ".")
        #expect(model.isFiltering)
        #expect(model.visibleRows == [])
    }

    @Test func fillMissingColumnsOnlyTouchesCriteriaWithoutAColumn() {
        let model = model(FilterCriterion(columnID: numberID), FilterCriterion())
        model.fillMissingColumns(with: nameID)
        #expect(model.criteria.map(\.columnID) == [numberID, nameID])
    }

    @Test func applyWithoutAnActiveCriterionLeavesNoFilter() {
        let model = model(FilterCriterion(columnID: nameID, op: .equals, value: ""))
        model.applyAll(to: table, decimalMark: ".")
        #expect(!model.isFiltering)
        #expect(model.visibleRows == nil)

        let id = model.criteria[0].id
        model.apply(criterionID: id, to: table, decimalMark: ".")
        #expect(!model.isFiltering)
        #expect(model.visibleRows == nil)
    }

    @Test func applyingOnlyInactiveCriteriaClearsAnAppliedFilter() {
        let model = model(FilterCriterion(columnID: nameID, op: .equals, value: "a"))
        model.applyAll(to: table, decimalMark: ".")
        #expect(model.visibleRows == [0, 2])

        model.criteria[0].value = ""
        model.applyAll(to: table, decimalMark: ".")
        #expect(!model.isFiltering)
        #expect(model.visibleRows == nil)
        #expect(model.criteria[0].columnID == nameID)
    }
}
