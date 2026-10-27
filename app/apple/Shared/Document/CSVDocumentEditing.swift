import Foundation
import TabToolKit

/// Table mutations exposed to the grid, menus, and format controls.
extension CSVDocument {
    /// Commits the grid's field editor and returns the current table.
    ///
    /// Structural actions must read the table only after this. A caller that
    /// derives a new table from a stale copy would otherwise replace the cell
    /// edit that the field editor just committed.
    func tableAfterEndingEditing() -> CSVTable? {
        endEditing()
        return table
    }

    /// Whether the current selection can be deleted.
    ///
    /// A column-only selection is disabled when the table has a single column:
    /// the document keeps that column, so the command would do nothing. The
    /// toolbar and the Edit menu both use this predicate.
    var canDeleteSelection: Bool {
        guard let table, isEditableContent else { return false }

        let rows = selection.rows.intersection(IndexSet(0..<table.rowCount))
        if !rows.isEmpty { return true }

        let columns = selection.columns.intersection(IndexSet(0..<table.columns.count))
        return !columns.isEmpty && table.columns.count > 1
    }

    /// Whether the table is loaded and structural commands may run.
    var isEditableContent: Bool {
        guard table != nil else { return false }
        switch phase {
        case .ready, .needsAttention:
            return true
        case .loading, .readFailed, .unreadableFile:
            return false
        }
    }

    /// Whether a valid row selection can be deleted.
    var canDeleteSelectedRows: Bool {
        guard let table, isEditableContent else { return false }
        return !selection.rows.intersection(IndexSet(0..<table.rowCount)).isEmpty
    }

    /// Whether a valid column selection can be deleted.
    var canDeleteSelectedColumns: Bool {
        guard let table, isEditableContent else { return false }
        let columns = selection.columns.intersection(IndexSet(0..<table.columns.count))
        return !columns.isEmpty && table.columns.count > 1
    }

    /// Inserts a blank row above the first selected row, or at the top.
    func addRowAboveSelection() {
        guard let table = tableAfterEndingEditing() else { return }
        insertRowSelecting(at: selection.rows.first ?? 0, rowCount: table.rowCount)
    }

    /// Inserts a blank row below the last selected row, or at the end.
    func addRowBelowSelection() {
        guard let table = tableAfterEndingEditing() else { return }
        let index = selection.rows.last.map { $0 + 1 } ?? table.rowCount
        insertRowSelecting(at: index, rowCount: table.rowCount)
    }

    /// Inserts a blank column left of the first selected column, or at the left.
    func addColumnLeftOfSelection() {
        guard let table = tableAfterEndingEditing() else { return }
        insertColumnSelecting(at: selection.columns.first ?? 0,
                              columnCount: table.columns.count)
    }

    /// Inserts a blank column right of the last selected column, or at the right.
    func addColumnRightOfSelection() {
        guard let table = tableAfterEndingEditing() else { return }
        let index = selection.columns.last.map { $0 + 1 } ?? table.columns.count
        insertColumnSelecting(at: index, columnCount: table.columns.count)
    }

    /// Deletes the selected rows and clears the selection.
    func deleteSelectedRows() {
        guard canDeleteSelectedRows else { return }
        deleteRows(at: selection.rows)
        selection = GridSelection()
    }

    /// Deletes the selected columns and clears the selection.
    func deleteSelectedColumns() {
        guard canDeleteSelectedColumns else { return }
        deleteColumns(at: selection.columns)
        selection = GridSelection()
    }

    private func insertRowSelecting(at index: Int, rowCount: Int) {
        let insertion = min(max(index, 0), rowCount)
        addRow(at: insertion)
        selection = GridSelection(rows: IndexSet(integer: insertion))
    }

    private func insertColumnSelecting(at index: Int, columnCount: Int) {
        let insertion = min(max(index, 0), columnCount)
        addColumn(at: insertion)
        selection = GridSelection(columns: IndexSet(integer: insertion))
    }

    /// Replaces one cell with a targeted undo record.
    ///
    /// Invalid indexes and unchanged values are ignored, so a no-op edit does
    /// not create an undo record.
    func setCell(_ text: String, row: Int, column: Int) {
        applyCellChange(text, row: row, column: column, actionName: "Edit Cell")
    }

    /// Inserts a blank row, clamped by the table model.
    func addRow(at index: Int) {
        guard let table = tableAfterEndingEditing() else { return }
        var updated = table
        updated.addRow(at: index)
        applyChange(table: updated, configuration: configuration,
                    includesUTF8BOM: hadUTF8BOM, actionName: "Add Row")
    }

    /// Removes the rows at `indexes`; invalid indexes are ignored.
    func deleteRows(at indexes: IndexSet) {
        guard let table = tableAfterEndingEditing() else { return }
        var updated = table
        updated.deleteRows(at: indexes)
        applyChange(table: updated, configuration: configuration,
                    includesUTF8BOM: hadUTF8BOM, actionName: "Delete Rows")
    }

    /// Moves the rows at `indexes` before `destination`.
    func moveRows(at indexes: IndexSet, to destination: Int) {
        guard let table = tableAfterEndingEditing() else { return }
        var updated = table
        updated.moveRows(at: indexes, to: destination)
        applyChange(table: updated, configuration: configuration,
                    includesUTF8BOM: hadUTF8BOM, actionName: "Move Rows")
    }

    /// Inserts a blank column, clamped by the table model.
    ///
    /// The new column gets an empty header only when header mode is enabled.
    func addColumn(at index: Int) {
        guard let table = tableAfterEndingEditing() else { return }
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
        guard let table = tableAfterEndingEditing() else { return }
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
        guard let table = tableAfterEndingEditing() else { return }
        var updated = table
        updated.moveColumn(from: from, to: to)
        applyChange(table: updated, configuration: configuration,
                    includesUTF8BOM: hadUTF8BOM, actionName: "Move Column",
                    sort: activeSort)
    }

    /// Converts numeric cells from the current mark to `mark` in place.
    ///
    /// Only numeric text changes; formatting such as leading zeros and
    /// exponents is preserved. The configuration follows the table so saving
    /// and pasting use the new mark.
    func setDecimalMark(_ mark: String) {
        guard !mark.isEmpty, mark != configuration.decimalMark else { return }
        guard let table = tableAfterEndingEditing() else { return }
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
        guard let table = tableAfterEndingEditing() else { return }
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
