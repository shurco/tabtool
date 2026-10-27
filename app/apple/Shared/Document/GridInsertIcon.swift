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
        Image(systemName: axis.symbolName)
            .font(.system(size: 16))
            .overlay(alignment: .bottomTrailing) {
                Image(systemName: "plus.circle.fill")
                    .font(.system(size: 8, weight: .bold))
                    .symbolRenderingMode(.palette)
                    .foregroundStyle(.white, .green)
                    .offset(x: 2, y: 2)
                    .accessibilityHidden(true)
            }
            .frame(width: 22, height: 20)
    }
}
