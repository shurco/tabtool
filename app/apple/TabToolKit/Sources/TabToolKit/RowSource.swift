import Foundation

/// A read-only view of tabular data that a grid can display.
///
/// `CSVTable` conforms for in-memory documents; large files conform through a
/// memory-mapped source that materializes only the rows being read.
public protocol RowSource: Sendable {
    var rowCount: Int { get }
    var columnCount: Int { get }
    var columnIDs: [UUID] { get }

    /// The column names, or `nil` when the data has no header row.
    var headers: [String]? { get }

    /// Returns the cell text, or an empty string for any invalid index.
    func cell(row: Int, column: Int) -> String
}

extension CSVTable: RowSource {
    public var columnCount: Int { columns.count }
    public var columnIDs: [UUID] { columns.map(\.id) }
}
