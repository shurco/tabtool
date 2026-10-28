import Foundation
import TabToolKit

/// A column offered by the filter bar's column picker.
struct FilterColumn: Identifiable, Equatable {
    let id: UUID
    let name: String

    /// The columns of `table`, named like the grid headers: by the header
    /// text, or by generated letters when the table has no header row.
    static func columns(of table: CSVTable, configuration: CSVConfiguration) -> [FilterColumn] {
        let presenter = GridPresenter(table: table, configuration: configuration)
        return table.columnIDs.enumerated().map { index, id in
            FilterColumn(id: id, name: presenter.header(at: index))
        }
    }
}
