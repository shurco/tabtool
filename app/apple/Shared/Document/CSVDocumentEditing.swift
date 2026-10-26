import Foundation
import TabToolKit

/// Table mutations exposed to the grid, menus, and format controls.
extension CSVDocument {
    /// Replaces one cell with a targeted undo record.
    ///
    /// Invalid indexes and unchanged values are ignored, so a no-op edit does
    /// not create an undo record.
    func setCell(_ text: String, row: Int, column: Int) {
        applyCellChange(text, row: row, column: column, actionName: "Edit Cell")
    }

    /// Inserts a blank row, clamped by the table model.
    func addRow(at index: Int) {
        guard let table else { return }
        var updated = table
        updated.addRow(at: index)
        applyChange(table: updated, configuration: configuration,
                    includesUTF8BOM: hadUTF8BOM, actionName: "Add Row")
    }

    /// Removes the rows at `indexes`; invalid indexes are ignored.
    func deleteRows(at indexes: IndexSet) {
        guard let table else { return }
        var updated = table
        updated.deleteRows(at: indexes)
        applyChange(table: updated, configuration: configuration,
                    includesUTF8BOM: hadUTF8BOM, actionName: "Delete Rows")
    }

    /// Moves the rows at `indexes` before `destination`.
    func moveRows(at indexes: IndexSet, to destination: Int) {
        guard let table else { return }
        var updated = table
        updated.moveRows(at: indexes, to: destination)
        applyChange(table: updated, configuration: configuration,
                    includesUTF8BOM: hadUTF8BOM, actionName: "Move Rows")
    }

    /// Inserts a blank column, clamped by the table model.
    ///
    /// The new column gets an empty header only when header mode is enabled.
    func addColumn(at index: Int) {
        guard let table else { return }
        var updated = table
        updated.addColumn(at: index,
                          header: configuration.firstRowAsHeader ? "" : nil)
        applyChange(table: updated, configuration: configuration,
                    includesUTF8BOM: hadUTF8BOM, actionName: "Add Column")
    }

    /// Removes the columns at `indexes`; invalid indexes are ignored.
    ///
    /// Deleting every column keeps the leftmost selected column, so the
    /// document and the grid always have at least one column.
    func deleteColumns(at indexes: IndexSet) {
        guard let table else { return }
        let selected = indexes.intersection(IndexSet(0..<table.columns.count))
        guard !selected.isEmpty else { return }

        var updated = table
        if selected.count == table.columns.count {
            var toDelete = selected
            toDelete.remove(selected.first ?? 0)
            updated.deleteColumns(at: toDelete)
        } else {
            updated.deleteColumns(at: selected)
        }
        applyChange(table: updated, configuration: configuration,
                    includesUTF8BOM: hadUTF8BOM, actionName: "Delete Columns")
    }

    /// Moves a column to another visual position.
    func moveColumn(from: Int, to: Int) {
        guard let table else { return }
        var updated = table
        updated.moveColumn(from: from, to: to)
        applyChange(table: updated, configuration: configuration,
                    includesUTF8BOM: hadUTF8BOM, actionName: "Move Column")
    }

    /// Converts numeric cells from the current mark to `mark` in place.
    ///
    /// Only numeric text changes; formatting such as leading zeros and
    /// exponents is preserved. The configuration follows the table so saving
    /// and pasting use the new mark.
    func setDecimalMark(_ mark: String) {
        guard !mark.isEmpty, mark != configuration.decimalMark,
              let table else {
            return
        }
        var updated = table
        updated.convertDecimalMark(from: configuration.decimalMark, to: mark)
        var newConfiguration = configuration
        newConfiguration.decimalMark = mark
        applyChange(table: updated, configuration: newConfiguration,
                    includesUTF8BOM: hadUTF8BOM, actionName: "Change Format")
    }

    /// Turns header mode on or off by moving the first row across the header
    /// boundary, updating both the table and the configuration.
    func setHeaderEnabled(_ enabled: Bool) {
        guard let table else { return }
        let hasHeaders = table.headers != nil

        if hasHeaders == enabled {
            guard configuration.firstRowAsHeader != enabled else { return }
            var newConfiguration = configuration
            newConfiguration.firstRowAsHeader = enabled
            applyChange(table: table, configuration: newConfiguration,
                        includesUTF8BOM: hadUTF8BOM, actionName: "Toggle Header")
            return
        }

        var updated = table
        if enabled {
            updated.setHeaders(table.columns.map { $0.cells.first ?? "" })
            updated.deleteRows(at: IndexSet(integer: 0))
        } else if let headers = table.headers {
            updated.setHeaders(nil)
            updated.addRow(at: 0)
            for (column, header) in headers.enumerated() {
                updated.setCell(header, row: 0, column: column)
            }
        }

        var newConfiguration = configuration
        newConfiguration.firstRowAsHeader = enabled
        applyChange(table: updated, configuration: newConfiguration,
                    includesUTF8BOM: hadUTF8BOM, actionName: "Toggle Header")
    }
}
