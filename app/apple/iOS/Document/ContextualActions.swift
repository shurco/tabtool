/// What the current grid selection contains.
enum ContextualScope: Equatable {
    case none, rows, columns
}

/// The buttons the contextual action bar can offer.
///
/// The raw values are the accessibility identifiers of the equivalent
/// controls.
enum ContextualActionID: String, CaseIterable {
    case addRowAbove = "add-row-above", addRowBelow = "add-row"
    case addColumnLeft = "add-column-left", addColumnRight = "add-column"
    case delete = "delete-selection"
    case sortAscending, sortDescending, removeSort
    case renameColumn = "rename-column"
}

/// Derives the contextual action bar's contents from the grid selection.
struct ContextualActions {
    static func scope(for selection: GridSelection) -> ContextualScope {
        if !selection.rows.isEmpty { return .rows }
        if !selection.columns.isEmpty { return .columns }
        return .none
    }

    /// The actions offered for `selection`, or none without a selection or
    /// when structural edits are unavailable (large file mode).
    static func ids(for selection: GridSelection, allowsStructuralEdits: Bool) -> [ContextualActionID] {
        guard allowsStructuralEdits else { return [] }
        switch scope(for: selection) {
        case .none:
            return []
        case .rows:
            return [.addRowAbove, .addRowBelow, .delete]
        case .columns:
            return [.addColumnLeft, .addColumnRight, .delete,
                    .sortAscending, .sortDescending, .removeSort, .renameColumn]
        }
    }
}
