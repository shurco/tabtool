import Foundation
import TabToolKit

/// One-step move commands for the selected rows or columns.
extension CSVDocument {
    /// Moves the selected rows by `offset`; the selection follows them.
    func moveSelectedRows(by offset: Int) {
        guard let table = tableAfterEndingEditing(),
              let order = rowMoveOrder(by: offset) else { return }

        var updated = table
        updated.reorderRows(order)
        applyChange(table: updated, configuration: configuration,
                    includesUTF8BOM: hadUTF8BOM, actionName: "Move Rows")
        selection = GridSelection(rows: shifted(selection.rows, by: offset))
    }

    /// Moves the selected columns by `offset`; the selection follows them.
    func moveSelectedColumns(by offset: Int) {
        guard let table = tableAfterEndingEditing(),
              let order = columnMoveOrder(by: offset) else { return }

        var updated = table
        updated.reorderColumns(order)
        applyChange(table: updated, configuration: configuration,
                    includesUTF8BOM: hadUTF8BOM, actionName: "Move Columns",
                    sort: activeSort)
        selection = GridSelection(columns: shifted(selection.columns, by: offset))
    }

    func canMoveSelectedRows(by offset: Int) -> Bool {
        rowMoveOrder(by: offset) != nil
    }

    func canMoveSelectedColumns(by offset: Int) -> Bool {
        columnMoveOrder(by: offset) != nil
    }

    private func rowMoveOrder(by offset: Int) -> [Int]? {
        moveOrder(count: table?.rowCount ?? 0, selected: selection.rows, by: offset)
    }

    private func columnMoveOrder(by offset: Int) -> [Int]? {
        moveOrder(count: table?.columns.count ?? 0, selected: selection.columns, by: offset)
    }

    /// The order that moves the valid part of `selected` by `offset`.
    private func moveOrder(count: Int, selected: IndexSet, by offset: Int) -> [Int]? {
        guard isEditableContent else { return nil }
        return CSVReorder.shifted(count: count,
                                  moving: selected.intersection(IndexSet(0..<count)),
                                  by: offset)
    }

    private func shifted(_ indexes: IndexSet, by offset: Int) -> IndexSet {
        IndexSet(indexes.map { $0 + offset })
    }
}
