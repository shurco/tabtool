import SwiftUI
import TabToolKit

/// One button in the contextual action bar.
struct ContextualAction: Identifiable {
    let id: ContextualActionID
    let title: LocalizedStringKey
    let systemImage: String
    let isEnabled: Bool
}

extension ContextualAction {
    init(id: ContextualActionID, isEnabled: Bool) {
        self.init(id: id, title: Self.title(for: id), systemImage: Self.systemImage(for: id),
                  isEnabled: isEnabled)
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
            bar
                .padding(.horizontal, 4)
                .frame(height: 44)
        }
        .background(.regularMaterial)
        .accessibilityElement(children: .contain)
    }

    /// A partially folded inner display puts the fold through the middle of a
    /// full-width bar. Keep the buttons on either side of it instead of letting
    /// the curve land on one.
    @ViewBuilder
    private var bar: some View {
        if #available(iOS 27.1, *) {
            GeometryReader { proxy in
                if let fold = FoldGap.in(proxy, actionCount: actions.count) {
                    HStack(spacing: 0) {
                        buttons(Array(actions.prefix(fold.split)))
                            .frame(width: fold.leading)
                        Color.clear.frame(width: fold.width)
                        buttons(Array(actions.dropFirst(fold.split)))
                            .frame(width: fold.trailing)
                    }
                } else {
                    buttons(actions)
                }
            }
        } else {
            buttons(actions)
        }
    }

    private func buttons(_ actions: [ContextualAction]) -> some View {
        HStack(spacing: 0) {
            ForEach(actions) { action in
                Button(role: action.id == .delete ? .destructive : nil) {
                    perform(action.id)
                } label: {
                    Image(systemName: action.systemImage)
                        .font(.system(size: 17))
                        .frame(minWidth: 44, minHeight: 44)
                        .frame(maxWidth: .infinity)
                        .contentShape(Rectangle())
                }
                .buttonStyle(.borderless)
                .tint(action.id == .delete ? .red : .accentColor)
                .disabled(!action.isEnabled)
                .accessibilityLabel(action.title)
                .accessibilityIdentifier(action.id.rawValue)
            }
        }
    }
}

/// The space a book-fold takes out of the bar, in the bar's own coordinates.
struct FoldGap {
    let leading: CGFloat
    let width: CGFloat
    let trailing: CGFloat
    let split: Int

    @available(iOS 27.1, *)
    static func `in`(_ proxy: GeometryProxy, actionCount: Int) -> FoldGap? {
        guard let fold = proxy.reservedRegions(kind: .division).first(where: {
            $0.frame.height > $0.frame.width
        }) else { return nil }
        return make(fold: fold.frame, totalWidth: proxy.size.width, actionCount: actionCount)
    }

    /// The bar only splits when the fold is active, vertical, and clear of both
    /// edges. A fold at an edge would leave no room for one of the groups.
    static func make(fold: CGRect, totalWidth: CGFloat, actionCount: Int) -> FoldGap? {
        guard actionCount > 1, fold.width > 0,
              fold.minX > 0, fold.maxX < totalWidth else { return nil }
        return FoldGap(leading: fold.minX,
                       width: fold.width,
                       trailing: totalWidth - fold.maxX,
                       split: (actionCount + 1) / 2)
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
