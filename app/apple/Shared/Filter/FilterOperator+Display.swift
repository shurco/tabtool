import TabToolKit

extension FilterOperator {
    /// The picker title: a symbol, or a `Localizable.xcstrings` key for the
    /// operators spelled out in words. Wrap it in `LocalizedStringKey`.
    var title: String {
        switch self {
        case .equals: return "="
        case .notEquals: return "≠"
        case .contains: return "Contains"
        case .notContains: return "Not contains"
        case .beginsWith: return "Begins with"
        case .endsWith: return "Ends with"
        case .greaterThan: return ">"
        case .lessThan: return "<"
        case .greaterOrEqual: return "≥"
        case .lessOrEqual: return "≤"
        case .isEmpty: return "Is empty"
        case .isNotEmpty: return "Is not empty"
        case .matchesRegex: return "Matches regex"
        }
    }
}
