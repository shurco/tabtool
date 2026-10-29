import SwiftUI

/// Placeholder until the collection-view grid lands. The initializer matches
/// the Mac grid so `DocumentView` stays shared.
struct CSVGrid: View {
    @ObservedObject var document: CSVDocument
    let isEditable: Bool
    let allowsStructuralEdits: Bool

    var body: some View {
        Color.clear
            .accessibilityIdentifier("grid")
    }
}
