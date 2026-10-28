import SwiftUI

/// Toolbar glyph for inserting a row or a column.
///
/// The filled half of the rectangle shows where the new row or column goes,
/// and the green badge marks the action as an insertion.
struct GridInsertIcon: View {
    enum Position {
        case rowAbove
        case rowBelow
        case columnLeft
        case columnRight

        var symbolName: String {
            switch self {
            case .rowAbove:
                return "rectangle.tophalf.inset.filled"
            case .rowBelow:
                return "rectangle.bottomhalf.inset.filled"
            case .columnLeft:
                return "rectangle.lefthalf.inset.filled"
            case .columnRight:
                return "rectangle.righthalf.inset.filled"
            }
        }
    }

    let position: Position

    var body: some View {
        GridToolbarGlyph(
            symbolName: position.symbolName,
            badge: .init(symbolName: "plus.circle.fill", color: .green)
        )
    }
}
