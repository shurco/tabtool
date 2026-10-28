import SwiftUI

/// Toolbar glyph for adding a row or a column.
///
/// The split rectangle names the axis, and the green badge marks the action as
/// an insertion, so the two add buttons no longer read as the same plus shape.
struct GridInsertIcon: View {
    enum Axis {
        case row
        case column

        var symbolName: String {
            switch self {
            case .row:
                return "rectangle.split.1x2"
            case .column:
                return "rectangle.split.3x1"
            }
        }
    }

    let axis: Axis

    var body: some View {
        GridToolbarGlyph(
            symbolName: axis.symbolName,
            badge: .init(symbolName: "plus.circle.fill", color: .green)
        )
    }
}
