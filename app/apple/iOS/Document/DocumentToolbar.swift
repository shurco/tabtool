import SwiftUI
import TabToolKit

#if os(iOS)
/// The document toolbar on iOS: history on the leading edge, filters, export,
/// and the overflow menu on the trailing edge.
extension DocumentView {
    @ToolbarContentBuilder
    var documentToolbar: some ToolbarContent {
        ToolbarItemGroup(placement: .topBarLeading) {
            if document.isLargeMode {
                Button("Save") { LargeSaveCoordinator.shared.save(document) }
                    .disabled(!document.hasUnsavedLargeEdits || document.largeStatus != .ready)
                    .accessibilityIdentifier("save")
                Button("Save As") { LargeSaveCoordinator.shared.saveAs(document) }
                    .disabled(document.largeStatus != .ready)
                    .accessibilityIdentifier("save-as")
            }
            Button("Undo", systemImage: "arrow.uturn.backward") { document.undoManager?.undo() }
                .disabled(!(document.undoManager?.canUndo ?? false))
                .accessibilityIdentifier("undo")
            Button("Redo", systemImage: "arrow.uturn.forward") { document.undoManager?.redo() }
                .disabled(!(document.undoManager?.canRedo ?? false))
                .accessibilityIdentifier("redo")
        }
        ToolbarItemGroup(placement: .topBarTrailing) {
            filterButton
            exportButton
            optionsMenu
        }
    }

    /// Shows or hides the filter sheet; a badge reports the applied criteria.
    private var filterButton: some View {
        Button {
            document.toggleFilterBar()
        } label: {
            Image(systemName: "magnifyingglass")
                .overlay(alignment: .topTrailing) {
                    if document.rowFilter.isFiltering {
                        Text(verbatim: "\(enabledCriteriaCount)")
                            .font(.caption2)
                            .foregroundStyle(.white)
                            .padding(2)
                            .background(Circle().fill(.tint))
                            .offset(x: 8, y: -8)
                    }
                }
        }
        .accessibilityLabel("Filter")
        .accessibilityIdentifier("filter-toggle")
        .disabled(!document.canFilter)
    }

    private var exportButton: some View {
        Button {
            ExportCoordinator.present(for: document)
        } label: {
            Image(systemName: "square.and.arrow.up")
        }
        .accessibilityLabel("Export…")
        .accessibilityIdentifier("export")
        .disabled(!document.isEditableContent)
    }

    private var optionsMenu: some View {
        Menu {
            Button("Format…") { showsFormatSheet = true }
            Button("Settings…") { showsSettings = true }
                .accessibilityIdentifier("settings")
            Button("About") { showsAbout = true }
            Divider()
            Button("Save As…") { LargeSaveCoordinator.shared.saveAs(document) }
                .disabled(document.largeStatus != .ready)
                .accessibilityIdentifier("save-as")
        } label: {
            Image(systemName: "ellipsis.circle")
        }
        .accessibilityLabel("More")
    }

    private var enabledCriteriaCount: Int {
        document.rowFilter.appliedFilter?.criteria.filter(\.isEnabled).count ?? 0
    }
}
#endif
