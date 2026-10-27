import Foundation

public enum CSVSortDirection: Sendable, Equatable {
    case ascending, descending
}

/// Sort keys for CSV cells: numbers, then natural-order text, then empties.
enum CSVCellOrder {
    enum Key {
        case number(Double)
        case text(String)
        case empty
    }

    static func key(for cell: String, decimalMark: String) -> Key {
        let trimmed = cell.trimmingCharacters(in: .whitespacesAndNewlines)
        if trimmed.isEmpty { return .empty }
        if CSVNumber.isNumeric(trimmed, decimalMark: decimalMark),
           let value = Double(trimmed.replacingOccurrences(of: decimalMark, with: ".")) {
            return .number(value)
        }
        return .text(cell)
    }

    /// Strict ordering; empty keys come last in both directions.
    static func precedes(_ lhs: Key, _ rhs: Key, direction: CSVSortDirection) -> Bool {
        switch (lhs, rhs) {
        case (.empty, _): return false
        case (_, .empty): return true
        default:
            let result = ascendingResult(lhs, rhs)
            return direction == .ascending ? result == .orderedAscending
                                           : result == .orderedDescending
        }
    }

    private static func ascendingResult(_ lhs: Key, _ rhs: Key) -> ComparisonResult {
        switch (lhs, rhs) {
        case let (.number(a), .number(b)):
            return a < b ? .orderedAscending : a > b ? .orderedDescending : .orderedSame
        case let (.text(a), .text(b)):
            return a.localizedStandardCompare(b)
        case (.number, _):
            return .orderedAscending
        default:
            return .orderedDescending
        }
    }
}

extension CSVTable {
    /// Row indexes ordered by `column`; an invalid column yields the identity
    /// order. Keys are computed once per row and ties keep the original order.
    public func rowOrder(sortedByColumn column: Int, direction: CSVSortDirection,
                         decimalMark: String) -> [Int] {
        guard columns.indices.contains(column) else { return Array(0..<rowCount) }

        let keys = columns[column].cells.map {
            CSVCellOrder.key(for: $0, decimalMark: decimalMark)
        }
        return keys.indices.sorted { lhs, rhs in
            if CSVCellOrder.precedes(keys[lhs], keys[rhs], direction: direction) { return true }
            if CSVCellOrder.precedes(keys[rhs], keys[lhs], direction: direction) { return false }
            return lhs < rhs
        }
    }
}
