import Foundation
import TabToolKit

/// The row sort currently applied to the table.
///
/// `sourceOrder[i]` is the pre-sort index of current row `i`, so removing the
/// sort is the inverse permutation. It stays valid only while the row
/// structure is unchanged; every structural change clears it.
struct ActiveSort: Equatable {
    let columnID: UUID
    let direction: CSVSortDirection
    let sourceOrder: [Int]
}

/// Sorting commands and their menu predicates.
extension CSVDocument {
    /// Sorts the rows by `column`, keeping the sort removable.
    ///
    /// A sort that leaves the rows unchanged records no undo step; the
    /// indicator is still set.
    func sortRows(column: Int, direction: CSVSortDirection) {
        guard isEditableContent, let table = tableAfterEndingEditing(),
              table.columns.indices.contains(column) else { return }

        let order = table.rowOrder(sortedByColumn: column, direction: direction,
                                   decimalMark: configuration.decimalMark)
        let previousSource = activeSort?.sourceOrder ?? Array(0..<table.rowCount)
        let newSort = ActiveSort(columnID: table.columns[column].id, direction: direction,
                                 sourceOrder: order.map { previousSource[$0] })

        var sorted = table
        sorted.reorderRows(order)
        applyChange(table: sorted, configuration: configuration,
                    includesUTF8BOM: hadUTF8BOM, actionName: "Sort Rows", sort: newSort)
        activeSort = newSort
        selection = GridSelection(columns: [column])
    }

    /// Restores the pre-sort row order and clears the sort.
    func removeSort() {
        guard isEditableContent, let sort = activeSort,
              let table = tableAfterEndingEditing() else { return }

        var inverse = Array(repeating: 0, count: sort.sourceOrder.count)
        for (current, source) in sort.sourceOrder.enumerated() { inverse[source] = current }

        var restored = table
        restored.reorderRows(inverse)
        applyChange(table: restored, configuration: configuration,
                    includesUTF8BOM: hadUTF8BOM, actionName: "Remove Sort")
        activeSort = nil
    }

    /// The sort direction of `column` when it is the sorted column.
    func sortDirection(forColumn column: Int) -> CSVSortDirection? {
        guard let table, table.columns.indices.contains(column),
              activeSort?.columnID == table.columns[column].id else { return nil }
        return activeSort?.direction
    }

    /// Whether exactly one existing column is selected.
    var canSortSelection: Bool {
        guard let table, isEditableContent else { return false }
        let columns = selection.columns.intersection(IndexSet(0..<table.columns.count))
        return columns.count == 1
    }

    var canRemoveSort: Bool {
        isEditableContent && activeSort != nil
    }
}
