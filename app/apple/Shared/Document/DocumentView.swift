import SwiftUI

/// The editor surface: the toolbar and the editable grid, with loading and read
/// errors as overlays owned by later shell tasks.
struct DocumentView: View {
    @ObservedObject var document: CSVDocument
    let fileURL: URL?

    @Environment(\.undoManager) private var undoManager

    var body: some View {
        Group {
            switch document.phase {
            case .loading:
                ProgressView("Loading…")
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
            case .ready, .needsAttention:
                if document.table != nil {
                    VStack(spacing: 0) {
                        GridToolbar(document: document)
                        Divider()
                        CSVGrid(document: document, isEditable: true)
                    }
                }
            case .readFailed, .unreadableFile:
                Text("The file could not be read.")
                    .foregroundStyle(.secondary)
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
            }
        }
        .frame(minWidth: 640, minHeight: 400)
        .onAppear {
            document.setUndoManager(undoManager)
        }
        .task {
            document.setFileURL(fileURL)
            await document.load()
        }
        .onChange(of: fileURL) { _, newValue in
            // Save As changes the URL without recreating this view.
            document.setFileURL(newValue)
        }
    }
}

/// Row and column actions above the grid.
///
/// The primary action inserts below or to the right of the selection; the
/// context menu offers the opposite side. A new row or column becomes the
/// selection.
private struct GridToolbar: View {
    @ObservedObject var document: CSVDocument

    var body: some View {
        HStack(spacing: 2) {
            Button(action: addRowBelow) {
                Label("Add Row Below", systemImage: "plus.rectangle")
                    .labelStyle(.iconOnly)
            }
            .help("Add Row Below")
            .accessibilityLabel("Add Row Below")
            .accessibilityIdentifier("add-row")
            .contextMenu {
                Button("Add Row Above") {
                    insertRow(at: rowInsertionAbove)
                }
            }

            Button(action: addColumnRight) {
                Label("Add Column Right", systemImage: "plus.rectangle.portrait")
                    .labelStyle(.iconOnly)
            }
            .help("Add Column Right")
            .accessibilityLabel("Add Column Right")
            .accessibilityIdentifier("add-column")
            .contextMenu {
                Button("Add Column Left") {
                    insertColumn(at: columnInsertionLeft)
                }
            }

            Divider()
                .frame(height: 16)
                .padding(.horizontal, 4)

            Button(role: .destructive, action: deleteSelection) {
                Label("Delete Selection", systemImage: "trash")
                    .labelStyle(.iconOnly)
            }
            .help("Delete Selected Rows or Columns")
            .accessibilityLabel("Delete Selection")
            .accessibilityIdentifier("delete-selection")
            .disabled(!document.selection.hasSelection)

            Spacer()
        }
        .padding(.horizontal, 8)
        .padding(.vertical, 4)
    }

    private var rowInsertionBelow: Int {
        document.selection.rows.last.map { $0 + 1 } ?? (document.table?.rowCount ?? 0)
    }

    private var rowInsertionAbove: Int {
        document.selection.rows.first ?? 0
    }

    private var columnInsertionRight: Int {
        document.selection.columns.last.map { $0 + 1 }
            ?? (document.table?.columns.count ?? 0)
    }

    private var columnInsertionLeft: Int {
        document.selection.columns.first ?? 0
    }

    private func addRowBelow() {
        insertRow(at: rowInsertionBelow)
    }

    private func addColumnRight() {
        insertColumn(at: columnInsertionRight)
    }

    private func insertRow(at index: Int) {
        guard let table = document.table else { return }
        let insertion = min(max(index, 0), table.rowCount)
        document.addRow(at: insertion)
        document.selection = GridSelection(rows: IndexSet(integer: insertion))
    }

    private func insertColumn(at index: Int) {
        guard let table = document.table else { return }
        let insertion = min(max(index, 0), table.columns.count)
        document.addColumn(at: insertion)
        document.selection = GridSelection(columns: IndexSet(integer: insertion))
    }

    private func deleteSelection() {
        document.deleteSelection()
    }
}
