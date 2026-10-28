import SwiftUI

/// Row and column actions above the grid.
///
/// The primary action inserts below or to the right of the selection; the
/// context menu offers the opposite side. A new row or column becomes the
/// selection.
struct GridToolbar: View {
    @ObservedObject var document: CSVDocument

    var body: some View {
        HStack(spacing: 2) {
            Button {
                document.addRowBelowSelection()
            } label: {
                GridInsertIcon(axis: .row)
            }
            .help(document.isLargeMode ? Text("Not available in large file mode.") : Text("Add Row Below"))
            .accessibilityLabel("Add Row Below")
            .accessibilityIdentifier("add-row")
            .disabled(!document.canInsertRows)
            .contextMenu {
                Button("Add Row Above") {
                    document.addRowAboveSelection()
                }
            }

            Button {
                document.addColumnRightOfSelection()
            } label: {
                GridInsertIcon(axis: .column)
            }
            .help(document.isLargeMode ? Text("Not available in large file mode.") : Text("Add Column Right"))
            .accessibilityLabel("Add Column Right")
            .accessibilityIdentifier("add-column")
            .disabled(document.isLargeMode)
            .contextMenu {
                Button("Add Column Left") {
                    document.addColumnLeftOfSelection()
                }
            }

            Divider()
                .frame(height: 16)
                .padding(.horizontal, 4)

            Button(role: .destructive) {
                guard document.canDeleteSelection else { return }
                document.deleteSelection()
            } label: {
                Label("Delete Selection", systemImage: "trash")
                    .labelStyle(.iconOnly)
            }
            .help(document.isLargeMode ? Text("Not available in large file mode.") : Text("Delete Selected Rows or Columns"))
            .accessibilityLabel("Delete Selection")
            .accessibilityIdentifier("delete-selection")
            .disabled(!document.canDeleteSelection)

            Divider()
                .frame(height: 16)
                .padding(.horizontal, 4)

            filterToggle

            Spacer()
        }
        .padding(.horizontal, 8)
        .padding(.vertical, 4)
    }

    /// Shows or hides the filter bar; filled while a filter is applied.
    private var filterToggle: some View {
        let title: LocalizedStringKey = document.rowFilter.isBarVisible ? "Hide Filter" : "Show Filter"
        return Button {
            document.toggleFilterBar()
        } label: {
            Label(title, systemImage: document.rowFilter.isFiltering
                  ? "line.3.horizontal.decrease.circle.fill"
                  : "line.3.horizontal.decrease.circle")
                .labelStyle(.iconOnly)
        }
        .foregroundStyle(document.rowFilter.isFiltering ? AnyShapeStyle(.tint) : AnyShapeStyle(.primary))
        .help(document.isLargeMode ? Text("Not available in large file mode.") : Text(title))
        .accessibilityLabel(title)
        .accessibilityIdentifier("filter-toggle")
        .disabled(!document.canFilter)
    }
}
