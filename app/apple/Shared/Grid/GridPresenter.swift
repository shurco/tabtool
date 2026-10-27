import AppKit
import TabToolKit

/// Read-only projection of a row source for the AppKit grid.
///
/// Headers come from the document when it has them and from
/// `ColumnNameGenerator` otherwise. Numeric cells are right-aligned; all other
/// cells are left-aligned.
struct GridPresenter {
    let table: any RowSource
    let configuration: CSVConfiguration

    var numberOfRows: Int {
        table.rowCount
    }

    var numberOfColumns: Int {
        table.columnCount
    }

    func header(at index: Int) -> String {
        guard (0..<table.columnCount).contains(index) else { return "" }
        return table.headers?[index] ?? ColumnNameGenerator.name(for: index)
    }

    func cell(row: Int, column: Int) -> String {
        table.cell(row: row, column: column)
    }

    func alignment(row: Int, column: Int) -> NSTextAlignment {
        CSVNumber.isNumeric(cell(row: row, column: column),
                            decimalMark: configuration.decimalMark) ? .right : .left
    }
}
