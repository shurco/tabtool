import AppKit

/// Row and column selection for the grid.
///
/// A table selection is either rows or an explicit header-driven column set.
/// The document keeps the selection; the coordinator mirrors it into AppKit
/// and draws column highlights through `CSVTableHeaderView` and cell views.
extension CSVGridCoordinator {
    func tableViewSelectionDidChange(_ notification: Notification) {
        guard !isSyncingSelection, let tableView else { return }
        let rows = tableView.selectedRowIndexes

        if rows.isEmpty {
            // Keep an explicit header selection when the user clicks outside
            // the rows; only clear when a row selection was actually removed.
            guard !document.selection.rows.isEmpty else { return }
            columnSelectionAnchor = nil
            document.selection = GridSelection()
        } else {
            columnSelectionAnchor = nil
            let selection = GridSelection(rows: rows)
            guard document.selection != selection else { return }
            document.selection = selection
        }
        syncSelectionDisplay()
    }

    /// Handles an explicit header click.
    ///
    /// A plain click selects one column, Command-click toggles a column, and
    /// Shift-click extends the range from the last anchor. Selecting a column
    /// clears row selection; the document selection is either rows or columns.
    func selectColumn(at index: Int, modifiers: NSEvent.ModifierFlags) {
        guard let tableView, index >= 0, index < presenter.numberOfColumns else { return }

        var columns = document.selection.rows.isEmpty ? columnSelection : IndexSet()

        if modifiers.contains(.command) {
            if columns.contains(index) {
                columns.remove(index)
            } else {
                columns.insert(index)
            }
            if columns.isEmpty {
                columnSelectionAnchor = nil
            } else if columnSelectionAnchor == nil {
                columnSelectionAnchor = index
            }
        } else if modifiers.contains(.shift),
                  let anchor = columnSelectionAnchor ?? columns.min() {
            columns.formUnion(IndexSet(min(anchor, index)...max(anchor, index)))
        } else {
            columns = IndexSet(integer: index)
            columnSelectionAnchor = index
        }

        isSyncingSelection = true
        tableView.deselectAll(nil)
        isSyncingSelection = false

        document.selection = columns.isEmpty ? GridSelection() : GridSelection(columns: columns)
        syncSelectionDisplay()
    }

    /// The valid column selection the document currently describes.
    private var columnSelection: IndexSet {
        guard document.selection.rows.isEmpty else { return IndexSet() }
        return document.selection.columns.intersection(IndexSet(0..<presenter.numberOfColumns))
    }

    /// Whether AppKit already shows the document selection.
    var selectionMatchesDocument: Bool {
        guard let tableView else { return true }
        let rows = document.selection.rows.intersection(IndexSet(0..<presenter.numberOfRows))
        if !rows.isEmpty {
            return tableView.selectedRowIndexes == rows
        }
        return tableView.selectedRowIndexes.isEmpty
            && (headerView?.selectedColumns ?? IndexSet()) == columnSelection
    }

    func applySelectionFromDocument() {
        guard let tableView else { return }
        let rows = document.selection.rows.intersection(IndexSet(0..<presenter.numberOfRows))

        isSyncingSelection = true
        if rows.isEmpty {
            tableView.deselectAll(nil)
        } else {
            tableView.selectRowIndexes(rows, byExtendingSelection: false)
        }
        isSyncingSelection = false

        columnSelectionAnchor = columnSelection.min()
        syncSelectionDisplay()
    }

    /// Shows the document's column selection in the header and cell views.
    func syncSelectionDisplay() {
        let columns = columnSelection
        let changed = displayedColumns.symmetricDifference(columns)
        displayedColumns = columns
        headerView?.select(columns)

        guard let tableView, !changed.isEmpty, presenter.numberOfRows > 0 else { return }
        tableView.reloadData(forRowIndexes: IndexSet(0..<presenter.numberOfRows),
                             columnIndexes: changed)
    }
}
