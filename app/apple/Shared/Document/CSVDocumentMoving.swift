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
                    includesUTF8BOM: hadUTF8BOM, actionName: "Move Columns")
        selection = GridSelection(columns: shifted(selection.columns, by: offset))
    }

    func canMoveSelectedRows(by offset: Int) -> Bool {
        rowMoveOrder(by: offset) != nil
    }

    func canMoveSelectedColumns(by offset: Int) -> Bool {
        columnMoveOrder(by: offset) != nil
    }

    private func rowMoveOrder(by offset: Int) -> [Int]? {
        guard let table, isEditableContent else { return nil }
        return CSVReorder.shifted(
            count: table.rowCount,
            moving: selection.rows.intersection(IndexSet(0..<table.rowCount)), by: offset)
    }

    private func columnMoveOrder(by offset: Int) -> [Int]? {
        guard let table, isEditableContent else { return nil }
        return CSVReorder.shifted(
            count: table.columns.count,
            moving: selection.columns.intersection(IndexSet(0..<table.columns.count)),
            by: offset)
    }

    private func shifted(_ indexes: IndexSet, by offset: Int) -> IndexSet {
        IndexSet(indexes.map { $0 + offset })
    }
}
