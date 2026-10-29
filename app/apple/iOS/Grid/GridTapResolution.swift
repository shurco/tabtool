/// The cell the keyboard and the editor act on. Rows are display rows.
struct GridFocus: Equatable {
    var displayRow: Int
    var column: Int
}

/// The grid element a tap landed on.
enum GridElementKind {
    case cell
    case columnHeader
    case rowNumber
    case corner
}

/// What a tap should do, given the element and the focused cell.
enum GridTapOutcome: Equatable {
    case selectCell
    case editCell
    case selectColumn
    case selectRowNumber
    case ignore
}

/// Decides the effect of a tap without UIKit, so the rule is testable.
enum GridTapResolution {
    static func resolve(element: GridElementKind,
                        focus: GridFocus?,
                        tapped: GridFocus?) -> GridTapOutcome {
        switch element {
        case .columnHeader:
            return .selectColumn
        case .rowNumber:
            return .selectRowNumber
        case .cell:
            guard let tapped else { return .ignore }
            return focus == tapped ? .editCell : .selectCell
        case .corner:
            return .ignore
        }
    }
}
