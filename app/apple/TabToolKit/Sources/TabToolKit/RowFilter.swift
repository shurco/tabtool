import Foundation

/// A set of criteria that a row must all satisfy (AND).
public struct RowFilter: Equatable, Sendable {
    public var criteria: [FilterCriterion]

    public init(criteria: [FilterCriterion] = []) {
        self.criteria = criteria
    }

    /// Ascending indexes of the rows of `source` that satisfy every active
    /// criterion; all rows when none is active.
    ///
    /// A criterion is skipped when it is disabled, has no known column, is an
    /// invalid regex, or has an empty value while its operator needs one.
    /// Numeric comparisons use `decimalMark`, like sorting does.
    public func matchingRows(in source: some RowSource, decimalMark: String = ".") -> [Int] {
        let matchers = activeMatchers(in: source, decimalMark: decimalMark)
        return (0..<source.rowCount).filter { row in
            matchers.allSatisfy { $0.matches(source.cell(row: row, column: $0.column)) }
        }
    }

    /// Whether at least one criterion would narrow the rows of `source`; an
    /// inactive filter is the same as no filter.
    public func isActive(in source: some RowSource, decimalMark: String = ".") -> Bool {
        !activeMatchers(in: source, decimalMark: decimalMark).isEmpty
    }

    private func activeMatchers(
        in source: some RowSource, decimalMark: String
    ) -> [(column: Int, matches: (String) -> Bool)] {
        let columnIDs = source.columnIDs
        return criteria.compactMap { criterion in
            guard criterion.isEnabled, !criterion.needsInput,
                  let id = criterion.columnID, let column = columnIDs.firstIndex(of: id),
                  let matches = CellMatcher.make(for: criterion, decimalMark: decimalMark)
            else { return nil }
            return (column, matches)
        }
    }
}

private extension FilterCriterion {
    /// `true` when the operator needs a value and the value is empty.
    var needsInput: Bool { op.needsValue && value.isEmpty }
}

/// Builds the per-cell predicate of a criterion; the value, the number and
/// the regex are parsed once here, not per cell. Returns `nil` for an invalid
/// regex.
private enum CellMatcher {
    static func make(for criterion: FilterCriterion, decimalMark: String) -> ((String) -> Bool)? {
        let value = criterion.value
        switch criterion.op {
        case .isEmpty: return { isBlank($0) }
        case .isNotEmpty: return { !isBlank($0) }
        case .contains: return { $0.range(of: value, options: .caseInsensitive) != nil }
        case .notContains: return { $0.range(of: value, options: .caseInsensitive) == nil }
        case .beginsWith:
            return { $0.range(of: value, options: [.caseInsensitive, .anchored]) != nil }
        case .endsWith:
            return { $0.range(of: value, options: [.caseInsensitive, .anchored, .backwards]) != nil }
        case .matchesRegex:
            guard let regex = criterion.regex else { return nil }
            return { regex.firstMatch(in: $0, range: NSRange($0.startIndex..., in: $0)) != nil }
        case .equals, .notEquals, .greaterThan, .lessThan, .greaterOrEqual, .lessOrEqual:
            let comparer = Comparer(value: value, decimalMark: decimalMark)
            return { cell in
                guard let order = comparer.order(of: cell) else { return criterion.op == .notEquals }
                switch criterion.op {
                case .equals: return order == .orderedSame
                case .notEquals: return order != .orderedSame
                case .greaterThan: return order == .orderedDescending
                case .lessThan: return order == .orderedAscending
                case .greaterOrEqual: return order != .orderedAscending
                default: return order != .orderedDescending
                }
            }
        }
    }

    private static func isBlank(_ cell: String) -> Bool {
        cell.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
    }
}

/// Orders cells against one criterion value: numerically when both sides are
/// numbers, otherwise as case-insensitive text.
private struct Comparer {
    let value: String
    let decimalMark: String
    private let number: Double?

    init(value: String, decimalMark: String) {
        self.value = value.trimmingCharacters(in: .whitespacesAndNewlines)
        self.decimalMark = decimalMark
        if case .number(let parsed) = CSVCellOrder.key(for: value, decimalMark: decimalMark) {
            number = parsed
        } else {
            number = nil
        }
    }

    /// How `cell` orders relative to the value; `nil` for a blank cell, which
    /// satisfies no comparison.
    func order(of cell: String) -> ComparisonResult? {
        switch CSVCellOrder.key(for: cell, decimalMark: decimalMark) {
        case .empty:
            return nil
        case .number(let parsed):
            guard let number else { break }
            return parsed < number ? .orderedAscending : parsed > number ? .orderedDescending : .orderedSame
        case .text:
            break
        }
        return cell.trimmingCharacters(in: .whitespacesAndNewlines)
            .compare(value, options: .caseInsensitive)
    }
}
