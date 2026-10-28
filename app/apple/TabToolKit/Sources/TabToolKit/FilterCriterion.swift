import Foundation

/// How a filter criterion compares a cell with its value.
public enum FilterOperator: String, CaseIterable, Sendable {
    case equals, notEquals
    case contains, notContains, beginsWith, endsWith
    case greaterThan, lessThan, greaterOrEqual, lessOrEqual
    case isEmpty, isNotEmpty
    case matchesRegex

    /// `false` for the emptiness operators, which ignore the criterion value.
    public var needsValue: Bool {
        self != .isEmpty && self != .isNotEmpty
    }
}

/// One row of the filter bar: a column, an operator and a value to compare.
public struct FilterCriterion: Identifiable, Equatable, Sendable {
    public var id: UUID
    public var isEnabled: Bool
    public var columnID: UUID?
    public var op: FilterOperator
    public var value: String

    public init(id: UUID = UUID(), isEnabled: Bool = true, columnID: UUID? = nil,
                op: FilterOperator = .contains, value: String = "") {
        self.id = id
        self.isEnabled = isEnabled
        self.columnID = columnID
        self.op = op
        self.value = value
    }

    /// `false` only for a regex operator whose non-empty pattern does not compile.
    public var isValid: Bool {
        op != .matchesRegex || value.isEmpty || regex != nil
    }

    /// The compiled case-insensitive pattern of a regex criterion.
    var regex: NSRegularExpression? {
        try? NSRegularExpression(pattern: value, options: .caseInsensitive)
    }
}
