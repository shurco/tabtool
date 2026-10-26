import SwiftUI

/// Placeholder editor surface: shows loading progress until the grid lands.
struct DocumentView: View {
    @ObservedObject var document: CSVDocument
    let fileURL: URL?

    @Environment(\.undoManager) private var undoManager

    var body: some View {
        Group {
            switch document.phase {
            case .loading:
                ProgressView("Loading…")
            case .ready, .needsAttention:
                Text(tableSummary)
                    .foregroundStyle(.secondary)
            case .readFailed, .unreadableFile:
                Text("The file could not be read.")
                    .foregroundStyle(.secondary)
            }
        }
        .frame(minWidth: 640, minHeight: 400)
        .padding()
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

    private var tableSummary: String {
        guard let table = document.table else { return "" }
        return "\(table.rowCount) rows × \(table.columns.count) columns"
    }
}
