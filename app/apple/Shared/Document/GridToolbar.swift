import SwiftUI

/// Row and column actions above the grid.
///
/// Inserting before the selection needs a selected row or column; inserting
/// after it appends at the end when nothing is selected. A new row or column
/// becomes the selection.
struct GridToolbar: View {
    @ObservedObject var document: CSVDocument

    var body: some View {
        HStack(spacing: 2) {
            insertButton(.rowAbove, title: "Add Row Above", id: "add-row-above",
                         isEnabled: document.canAddRowAboveSelection, action: document.addRowAboveSelection)
            insertButton(.rowBelow, title: "Add Row Below", id: "add-row",
                         isEnabled: document.canInsertRows, action: document.addRowBelowSelection)

            Divider()
                .frame(height: 16)
                .padding(.horizontal, 4)

            insertButton(.columnLeft, title: "Add Column Left", id: "add-column-left",
                         isEnabled: document.canAddColumnLeftOfSelection,
                         action: document.addColumnLeftOfSelection)
            insertButton(.columnRight, title: "Add Column Right", id: "add-column",
                         isEnabled: !document.isLargeMode, action: document.addColumnRightOfSelection)

            Divider()
                .frame(height: 16)
                .padding(.horizontal, 4)

            Button(role: .destructive) {
                guard document.canDeleteSelection else { return }
                document.deleteSelection()
            } label: {
                GridToolbarGlyph(symbolName: "trash", pointSize: 13)
            }
            .help(document.isLargeMode ? Text("Not available in large file mode.") : Text("Delete Selected Rows or Columns"))
            .accessibilityLabel("Delete Selection")
            .accessibilityIdentifier("delete-selection")
            .disabled(!document.canDeleteSelection)

            Spacer()

            filterToggle
        }
        .padding(.horizontal, 8)
        .padding(.vertical, 4)
    }

    private func insertButton(_ position: GridInsertIcon.Position, title: LocalizedStringKey, id: String,
                              isEnabled: Bool, action: @escaping @MainActor () -> Void) -> some View {
        Button(action: action) {
            GridInsertIcon(position: position)
        }
        .help(document.isLargeMode ? Text("Not available in large file mode.") : Text(title))
        .accessibilityLabel(title)
        .accessibilityIdentifier(id)
        .disabled(!isEnabled)
    }

    /// Shows or hides the filter bar; blue with a light glyph while the bar
    /// is open.
    private var filterToggle: some View {
        let title: LocalizedStringKey = document.rowFilter.isBarVisible ? "Hide Filter" : "Show Filter"
        return Group {
            let button = Button {
                document.toggleFilterBar()
            } label: {
                GridToolbarGlyph(symbolName: "magnifyingglass", pointSize: 15)
            }
            if document.rowFilter.isBarVisible {
                button.buttonStyle(.borderedProminent)
            } else {
                button
            }
        }
        .help(document.isLargeMode ? Text("Not available in large file mode.") : Text(title))
        .accessibilityLabel(title)
        .accessibilityIdentifier("filter-toggle")
        .disabled(!document.canFilter)
    }
}
