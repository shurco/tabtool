import SwiftUI
import TabToolKit

/// One button in the contextual action bar.
struct ContextualAction: Identifiable {
    let id: ContextualActionID
    let title: LocalizedStringKey
    let systemImage: String
    let isEnabled: Bool
    let isDestructive: Bool
}

extension ContextualAction {
    init(id: ContextualActionID, isEnabled: Bool) {
        self.init(id: id, title: Self.title(for: id), systemImage: Self.systemImage(for: id),
                  isEnabled: isEnabled, isDestructive: id == .delete)
    }

    private static func title(for id: ContextualActionID) -> LocalizedStringKey {
        switch id {
        case .addRowAbove: "Add Row Above"
        case .addRowBelow: "Add Row Below"
        case .addColumnLeft: "Add Column Left"
        case .addColumnRight: "Add Column Right"
        case .delete: "Delete Selection"
        case .sortAscending: "Sort Ascending"
        case .sortDescending: "Sort Descending"
        case .removeSort: "Remove Sort"
        case .renameColumn: "Rename Column"
        }
    }

    private static func systemImage(for id: ContextualActionID) -> String {
        switch id {
        case .addRowAbove: "arrow.up.to.line"
        case .addRowBelow: "arrow.down.to.line"
        case .addColumnLeft: "arrow.left.to.line"
        case .addColumnRight: "arrow.right.to.line"
        case .delete: "trash"
        case .sortAscending: "arrow.up"
        case .sortDescending: "arrow.down"
        case .removeSort: "arrow.uturn.backward"
        case .renameColumn: "pencil"
        }
    }
}

/// The selection-aware action bar above the status line.
///
/// The view holds no document logic: the caller passes the descriptors and
/// receives the chosen action.
struct ContextualActionBar: View {
    let actions: [ContextualAction]
    let perform: (ContextualActionID) -> Void

    var body: some View {
        VStack(spacing: 0) {
            Divider()
            HStack(spacing: 0) {
                ForEach(actions) { action in
                    Button(role: action.isDestructive ? .destructive : nil) {
                        perform(action.id)
                    } label: {
                        Image(systemName: action.systemImage)
                            .font(.system(size: 17))
                            .frame(minWidth: 44, minHeight: 44)
                            .frame(maxWidth: .infinity)
                            .contentShape(Rectangle())
                    }
                    .buttonStyle(.borderless)
                    .tint(action.isDestructive ? .red : .accentColor)
                    .disabled(!action.isEnabled)
                    .accessibilityLabel(action.title)
                    .accessibilityIdentifier(action.id.rawValue)
                }
            }
            .padding(.horizontal, 4)
            .frame(height: 44)
        }
        .background(.regularMaterial)
        .accessibilityElement(children: .contain)
    }
}

extension ContextualActions {
    /// The bar's descriptors for the document's current selection.
    @MainActor
    static func actions(for document: CSVDocument) -> [ContextualAction] {
        ids(for: document.selection, allowsStructuralEdits: !document.isLargeMode)
            .map { ContextualAction(id: $0, isEnabled: isEnabled($0, in: document)) }
    }

    /// Whether the document can run `id` right now.
    @MainActor
    static func isEnabled(_ id: ContextualActionID, in document: CSVDocument) -> Bool {
        switch id {
        case .addRowAbove: document.canAddRowAboveSelection
        case .addRowBelow: document.canInsertRows
        case .addColumnLeft: document.canAddColumnLeftOfSelection
        case .addColumnRight: document.isEditableContent && !document.isLargeMode
        case .delete: document.canDeleteSelection
        case .sortAscending, .sortDescending: document.canSortSelection
        case .removeSort: document.canRemoveSort
        case .renameColumn: document.canRenameSelectedColumn
        }
    }

    /// Runs `id` against `document`; renaming is presented by the caller.
    @MainActor
    static func run(_ id: ContextualActionID, on document: CSVDocument, rename: () -> Void) {
        switch id {
        case .addRowAbove:
            document.addRowAboveSelection()
        case .addRowBelow:
            document.addRowBelowSelection()
        case .addColumnLeft:
            document.addColumnLeftOfSelection()
        case .addColumnRight:
            document.addColumnRightOfSelection()
        case .delete:
            document.deleteSelection()
        case .sortAscending:
            if let column = document.selection.columns.first {
                document.sortRows(column: column, direction: .ascending)
            }
        case .sortDescending:
            if let column = document.selection.columns.first {
                document.sortRows(column: column, direction: .descending)
            }
        case .removeSort:
            document.removeSort()
        case .renameColumn:
            rename()
        }
    }
}
