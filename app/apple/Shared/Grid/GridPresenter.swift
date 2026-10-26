import AppKit
import TabToolKit

/// Read-only projection of a CSV table for the AppKit grid.
///
/// Headers come from the document when it has them and from
/// `ColumnNameGenerator` otherwise. Numeric cells are right-aligned; all other
/// cells are left-aligned.
struct GridPresenter {
    let table: CSVTable
    let configuration: CSVConfiguration

    var numberOfRows: Int {
        table.rowCount
    }

    var numberOfColumns: Int {
        table.columns.count
    }

    func header(at index: Int) -> String {
        guard table.columns.indices.contains(index) else { return "" }
        return table.columns[index].header ?? ColumnNameGenerator.name(for: index)
    }

    func cell(row: Int, column: Int) -> String {
        table.cell(row: row, column: column)
    }

    func alignment(row: Int, column: Int) -> NSTextAlignment {
        CSVNumber.isNumeric(cell(row: row, column: column),
                            decimalMark: configuration.decimalMark) ? .right : .left
    }
}
