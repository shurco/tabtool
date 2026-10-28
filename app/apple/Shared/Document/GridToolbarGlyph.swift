import SwiftUI

/// Single glyph style shared by every grid toolbar button: one symbol size,
/// one frame, and an optional corner badge.
struct GridToolbarGlyph: View {
    struct Badge {
        let symbolName: String
        let color: Color
    }

    let symbolName: String
    var badge: Badge?

    var body: some View {
        Image(systemName: symbolName)
            .font(.system(size: 16))
            .overlay(alignment: .bottomTrailing) {
                if let badge {
                    Image(systemName: badge.symbolName)
                        .font(.system(size: 8, weight: .bold))
                        .symbolRenderingMode(.palette)
                        .foregroundStyle(.white, badge.color)
                        .offset(x: 2, y: 2)
                        .accessibilityHidden(true)
                }
            }
            .frame(width: 22, height: 20)
    }
}
