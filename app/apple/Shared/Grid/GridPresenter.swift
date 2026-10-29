#if canImport(AppKit)
import AppKit
#else
import UIKit
#endif
import TabToolKit

/// Read-only projection of a row source for the AppKit grid.
///
/// Headers come from the document when it has them and from
/// `ColumnNameGenerator` otherwise. Numeric cells are right-aligned; all other
/// cells are left-aligned.
///
/// With `visibleRows` the grid shows only those rows. Row arguments of
/// `cell` and `alignment` and the row count are display indexes, while the
/// document keeps model indexes; the mapping helpers convert between the two.
struct GridPresenter {
    let table: any RowSource
    let configuration: CSVConfiguration
    /// Ascending model indexes of the displayed rows, or `nil` for all rows.
    var visibleRows: [Int]? = nil

    var numberOfRows: Int {
        visibleRows?.count ?? table.rowCount
    }

    var numberOfColumns: Int {
        table.columnCount
    }

    func header(at index: Int) -> String {
        guard (0..<table.columnCount).contains(index) else { return "" }
        return table.headers?[index] ?? ColumnNameGenerator.name(for: index)
    }

    func cell(row: Int, column: Int) -> String {
        modelRow(forDisplayRow: row).map { table.cell(row: $0, column: column) } ?? ""
    }

    func alignment(row: Int, column: Int) -> NSTextAlignment {
        CSVNumber.isNumeric(cell(row: row, column: column),
                            decimalMark: configuration.decimalMark) ? .right : .left
    }

    // MARK: - Row mapping

    /// The model row shown at display row `row`, or `nil` for a display row
    /// the filter does not have.
    func modelRow(forDisplayRow row: Int) -> Int? {
        guard let visibleRows else { return row }
        return visibleRows.indices.contains(row) ? visibleRows[row] : nil
    }

    /// The display row of model row `row`, or `nil` when it is hidden.
    func displayRow(forModelRow row: Int) -> Int? {
        guard let visibleRows else { return row < table.rowCount ? row : nil }
        var low = 0, high = visibleRows.count
        while low < high {
            let middle = (low + high) / 2
            if visibleRows[middle] < row { low = middle + 1 } else { high = middle }
        }
        return low < visibleRows.count && visibleRows[low] == row ? low : nil
    }

    func modelRows(forDisplayRows rows: IndexSet) -> IndexSet {
        guard visibleRows != nil else { return rows }
        return IndexSet(rows.compactMap { modelRow(forDisplayRow: $0) })
    }

    /// The display rows of the given model rows; hidden rows are dropped.
    func displayRows(forModelRows rows: IndexSet) -> IndexSet {
        guard visibleRows != nil else {
            return rows.intersection(IndexSet(integersIn: 0..<table.rowCount))
        }
        return IndexSet(rows.compactMap { displayRow(forModelRow: $0) })
    }
}
