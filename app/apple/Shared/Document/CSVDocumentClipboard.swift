import Foundation
import TabToolKit

/// Copy, paste, and delete operations for the current grid selection.
extension CSVDocument {
    /// Rows selected in visual column order, or columns selected across all
    /// rows, as tab-separated text. Returns `nil` when nothing is selected.
    ///
    /// Numeric cells are localized to `locale`; text is copied as is, with
    /// tabs and line breaks inside cells flattened by `CSVClipboard`.
    func copySelection(locale: Locale = .current) -> String? {
        guard let table else { return nil }

        let rows: [[String]]
        if !selection.rows.isEmpty {
            let selected = selection.rows.intersection(IndexSet(0..<table.rowCount))
            guard !selected.isEmpty else { return nil }
            rows = selected.map { row in
                table.columns.map { $0.cells[row] }
            }
        } else if !selection.columns.isEmpty {
            let selected = selection.columns.intersection(IndexSet(0..<table.columns.count))
            guard !selected.isEmpty else { return nil }
            rows = (0..<table.rowCount).map { row in
                selected.map { table.columns[$0].cells[row] }
            }
        } else {
            return nil
        }

        return CSVClipboard.makeTSV(rows: rows,
                                    numericDecimalMark: configuration.decimalMark,
                                    locale: locale)
    }

    /// Inserts a TSV block after the last selected row (or at the end).
    ///
    /// Extra fields add columns on the right. Numeric cells are converted from
    /// the locale decimal separator to the document mark. The inserted rows
    /// become the selection and the whole paste is one undo operation.
    func pasteTSV(_ string: String, locale: Locale = .current) {
        guard let table else { return }

        let parsedRows = CSVClipboard.parseTSV(string)
        let fieldCount = parsedRows.map(\.count).max() ?? 0
        guard fieldCount > 0 else { return }

        var updated = table
        if fieldCount > updated.columns.count {
            for _ in updated.columns.count..<fieldCount {
                updated.addColumn(at: updated.columns.count,
                                  header: configuration.firstRowAsHeader ? "" : nil)
            }
        }

        let requestedInsertion = selection.rows.last.map { $0 + 1 } ?? updated.rowCount
        let insertion = min(max(requestedInsertion, 0), updated.rowCount)
        for offset in parsedRows.indices {
            updated.addRow(at: insertion + offset)
        }

        let sourceMark = locale.decimalSeparator ?? configuration.decimalMark
        for (rowOffset, fields) in parsedRows.enumerated() {
            for (column, field) in fields.enumerated() {
                let converted = DecimalMarkConverter.convert(
                    field, from: sourceMark, to: configuration.decimalMark
                )
                updated.setCell(converted, row: insertion + rowOffset, column: column)
            }
        }

        applyChange(table: updated, configuration: configuration,
                    includesUTF8BOM: hadUTF8BOM, actionName: "Paste Data")
        selection = GridSelection(rows: IndexSet(insertion..<(insertion + parsedRows.count)))
    }

    /// Deletes selected rows, or selected columns when no rows are selected.
    ///
    /// The selection is cleared because the deleted indexes no longer exist.
    func deleteSelection() {
        guard let table else { return }

        if !selection.rows.isEmpty {
            let selected = selection.rows.intersection(IndexSet(0..<table.rowCount))
            guard !selected.isEmpty else { return }
            deleteRows(at: selected)
            selection = GridSelection()
            return
        }

        guard !selection.columns.isEmpty else { return }
        let selected = selection.columns.intersection(IndexSet(0..<table.columns.count))
        guard !selected.isEmpty else { return }
        // `deleteColumns` keeps at least one column when every column is
        // selected, so the grid never drops to zero columns.
        deleteColumns(at: selected)
        selection = GridSelection()
    }
}
