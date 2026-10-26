import SwiftUI

/// The editor surface: the editable grid, with loading and read errors as
/// overlays owned by later shell tasks.
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
                    CSVGrid(document: document, isEditable: true)
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
