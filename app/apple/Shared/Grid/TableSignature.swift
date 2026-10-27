import Foundation

/// The table's shape and column identity, used to distinguish a cell edit from
/// a structural change.
struct TableSignature: Equatable {
    let rowCount: Int
    let columnCount: Int
    let columnIDs: [UUID]
    let headers: [String]?

    static let empty = TableSignature(rowCount: 0, columnCount: 0,
                                      columnIDs: [], headers: nil)

    init(rowCount: Int, columnCount: Int, columnIDs: [UUID], headers: [String]?) {
        self.rowCount = rowCount
        self.columnCount = columnCount
        self.columnIDs = columnIDs
        self.headers = headers
    }

    init(presenter: GridPresenter) {
        self.init(rowCount: presenter.numberOfRows,
                  columnCount: presenter.numberOfColumns,
                  columnIDs: presenter.table.columnIDs,
                  headers: presenter.table.headers)
    }
}
