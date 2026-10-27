import Foundation

/// One column of a `CSVTable`.
public struct CSVColumn: Identifiable, Sendable, Equatable {
    public let id: UUID
    public var header: String?
    public var cells: [String]

    public init(id: UUID = UUID(), header: String?, cells: [String]) {
        self.id = id
        self.header = header
        self.cells = cells
    }
}

/// A column-oriented CSV table.
///
/// Every column always holds exactly `rowCount` cells. Initializers pad ragged
/// input with empty strings and never truncate it. Headers are either present
/// on every column or absent from every column.
public struct CSVTable: Sendable, Equatable {
    public var columns: [CSVColumn]

    /// The number of cells in each column.
    public var rowCount: Int {
        columns.first?.cells.count ?? 0
    }

    /// The headers of all columns, or `nil` when the table has no headers.
    public var headers: [String]? {
        get {
            guard !columns.isEmpty, columns.allSatisfy({ $0.header != nil }) else {
                return nil
            }
            return columns.map { $0.header ?? "" }
        }
        set {
            setHeaders(newValue)
        }
    }

    /// Creates a table from columns, padding short columns with empty cells.
    ///
    /// A partially present set of headers is discarded as a whole so headers
    /// stay all-or-nothing.
    public init(columns: [CSVColumn]) {
        let rowCount = columns.map(\.cells.count).max() ?? 0
        var columns = columns

        for index in columns.indices where columns[index].cells.count < rowCount {
            let missing = rowCount - columns[index].cells.count
            columns[index].cells.append(contentsOf: repeatElement("", count: missing))
        }
        if columns.contains(where: { $0.header == nil }) {
            for index in columns.indices {
                columns[index].header = nil
            }
        }
        self.columns = columns
    }

    /// Creates a table from rows, padding every row to the widest row.
    ///
    /// `minimumColumnCount` guarantees at least that many columns. Headers also
    /// reserve columns, and missing names are padded with empty strings so
    /// headers stay all-or-nothing.
    public init(rows: [[String]], headers: [String]?, minimumColumnCount: Int = 1) {
        let width = max(minimumColumnCount, rows.map(\.count).max() ?? 0,
                        headers?.count ?? 0)
        let columnCount = max(0, width)
        var columns: [CSVColumn] = []
        columns.reserveCapacity(columnCount)

        for columnIndex in 0..<columnCount {
            let cells = rows.map { row in
                columnIndex < row.count ? row[columnIndex] : ""
            }
            let header = headers.map { names in
                columnIndex < names.count ? names[columnIndex] : ""
            }
            columns.append(CSVColumn(header: header, cells: cells))
        }
        self.init(columns: columns)
    }

    /// Compares document content, ignoring the session identity of columns.
    ///
    /// Two tables with the same headers and cell values are equal even when
    /// their columns carry different `UUID`s, as happens after re-reading a
    /// file. Use `hasSameColumnOrder(as:)` when identity or visual order must
    /// also be checked.
    public static func == (lhs: CSVTable, rhs: CSVTable) -> Bool {
        lhs.headers == rhs.headers
            && lhs.columns.map(\.cells) == rhs.columns.map(\.cells)
    }

    /// Whether both tables hold the same column identities in the same order.
    ///
    /// Content equality alone cannot detect a column move when the moved
    /// columns contain identical values, so document mutations also compare
    /// this. Re-reading a file produces new identities and is therefore a real
    /// state change even when the parsed content matches.
    public func hasSameColumnOrder(as other: CSVTable) -> Bool {
        columns.map(\.id) == other.columns.map(\.id)
    }

    /// Creates a blank document with at least one row and one column.
    public static func newDocument(rows: Int = 1, columns: Int = 3) -> CSVTable {
        let rowCount = max(1, rows)
        let columnCount = max(1, columns)
        return CSVTable(rows: Array(repeating: [String](), count: rowCount),
                        headers: nil,
                        minimumColumnCount: columnCount)
    }

    /// Returns the cell text, or an empty string when either index is invalid.
    public func cell(row: Int, column: Int) -> String {
        guard columns.indices.contains(column) else { return "" }
        let cells = columns[column].cells
        guard cells.indices.contains(row) else { return "" }
        return cells[row]
    }

    /// Replaces a cell's text, ignoring invalid indexes.
    public mutating func setCell(_ value: String, row: Int, column: Int) {
        guard columns.indices.contains(column),
              columns[column].cells.indices.contains(row) else {
            return
        }
        columns[column].cells[row] = value
    }

    /// Replaces all headers, or removes them when `headers` is `nil`.
    ///
    /// Missing names become empty strings; extra names are ignored.
    public mutating func setHeaders(_ headers: [String]?) {
        guard let headers else {
            for index in columns.indices {
                columns[index].header = nil
            }
            return
        }

        for index in columns.indices {
            columns[index].header = index < headers.count ? headers[index] : ""
        }
    }

    /// Inserts a blank row, clamping the index to the table bounds.
    public mutating func addRow(at index: Int) {
        guard !columns.isEmpty else { return }
        let insertion = min(max(index, 0), rowCount)

        for columnIndex in columns.indices {
            let safeInsertion = min(insertion, columns[columnIndex].cells.count)
            columns[columnIndex].cells.insert("", at: safeInsertion)
        }
    }

    /// Removes the rows at `indexes`; out-of-range indexes are ignored.
    ///
    /// Every column is rebuilt in one pass. Removing selected indexes one by
    /// one costs O(rows × selected) and is too slow for large selection on a
    /// 100k-row table.
    public mutating func deleteRows(at indexes: IndexSet) {
        let selected = indexes.intersection(IndexSet(0..<rowCount))
        guard !selected.isEmpty else { return }

        let order = (0..<rowCount).filter { !selected.contains($0) }
        applyRowOrder(order)
    }

    /// Moves the rows at `indexes` so they end up before `destination`.
    ///
    /// `destination` is an index in the pre-move table, clamped to the table
    /// bounds. The insertion point is adjusted for selected rows removed above
    /// it, and moved rows keep their original order. Each column is rebuilt in
    /// one pass so a large move stays linear in the table size.
    public mutating func moveRows(at indexes: IndexSet, to destination: Int) {
        let selected = indexes.intersection(IndexSet(0..<rowCount))
        guard !selected.isEmpty else { return }

        let destination = min(max(destination, 0), rowCount)
        let insertion = destination - selected.filter { $0 < destination }.count

        var remaining: [Int] = []
        remaining.reserveCapacity(rowCount - selected.count)
        var moving: [Int] = []
        moving.reserveCapacity(selected.count)
        for row in 0..<rowCount {
            if selected.contains(row) {
                moving.append(row)
            } else {
                remaining.append(row)
            }
        }
        let target = min(max(insertion, 0), remaining.count)
        remaining.insert(contentsOf: moving, at: target)
        applyRowOrder(remaining)
    }

    /// Reorders rows so result row `i` is current row `order[i]`.
    ///
    /// Input that is not a permutation of the row indexes is ignored.
    public mutating func reorderRows(_ order: [Int]) {
        guard CSVReorder.isPermutation(order, count: rowCount) else { return }
        applyRowOrder(order)
    }

    /// Reorders columns so result column `i` is current column `order[i]`.
    ///
    /// Input that is not a permutation of the column indexes is ignored.
    public mutating func reorderColumns(_ order: [Int]) {
        guard CSVReorder.isPermutation(order, count: columns.count) else { return }
        columns = order.map { columns[$0] }
    }

    /// Rebuilds every column from a row-index order in one pass.
    private mutating func applyRowOrder(_ order: [Int]) {
        for columnIndex in columns.indices {
            let cells = columns[columnIndex].cells
            columns[columnIndex].cells = order.map { cells[$0] }
        }
    }

    /// Inserts a blank column, clamping the index to the table bounds.
    ///
    /// `header` is used only when the table already has headers; otherwise the
    /// new column keeps headers disabled.
    public mutating func addColumn(at index: Int, header: String? = nil) {
        let insertion = min(max(index, 0), columns.count)
        let headerEnabled = headers != nil
        let column = CSVColumn(header: headerEnabled ? header ?? "" : nil,
                               cells: Array(repeating: "", count: rowCount))
        columns.insert(column, at: insertion)
    }

    /// Removes the columns at `indexes`; out-of-range indexes are ignored.
    public mutating func deleteColumns(at indexes: IndexSet) {
        let selected = indexes.intersection(IndexSet(0..<columns.count))
        for index in selected.reversed() {
            columns.remove(at: index)
        }
    }

    /// Moves a column to a new position, clamping both indexes.
    public mutating func moveColumn(from source: Int, to destination: Int) {
        guard columns.indices.contains(source) else { return }

        let target = min(max(destination, 0), columns.count - 1)
        let column = columns.remove(at: source)
        columns.insert(column, at: target)
    }

    /// Converts the decimal mark in numeric cells from `source` to `target`.
    public mutating func convertDecimalMark(from source: String, to target: String) {
        guard !source.isEmpty, source != target else { return }

        for columnIndex in columns.indices {
            for row in columns[columnIndex].cells.indices {
                let cell = columns[columnIndex].cells[row]
                guard CSVNumber.isNumeric(cell, decimalMark: source) else { continue }
                columns[columnIndex].cells[row] = cell.replacingOccurrences(
                    of: source, with: target
                )
            }
        }
    }
}
