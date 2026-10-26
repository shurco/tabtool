import AppKit

/// Clipboard, deletion, and select-all actions exposed by the grid.
///
/// The coordinator routes responder-chain actions here; the document remains
/// the only owner of the table and of the undo registration.
extension CSVGridCoordinator {
    /// Whether the current selection has rows or columns to copy.
    var canCopySelection: Bool {
        guard let table = document.table else { return false }
        let rows = document.selection.rows.intersection(IndexSet(0..<table.rowCount))
        let columns = document.selection.columns
            .intersection(IndexSet(0..<table.columns.count))
        return !rows.isEmpty || !columns.isEmpty
    }

    /// Whether the pasteboard currently provides text for a paste.
    var canPaste: Bool {
        isEditable
            && NSPasteboard.general.availableType(from: [.string, .tabularText]) != nil
    }

    /// Whether the current selection can be deleted.
    var canDeleteSelection: Bool {
        isEditable && document.canDeleteSelection
    }

    /// Whether the table has rows to select.
    var canSelectAll: Bool {
        (document.table?.rowCount ?? 0) > 0
    }

    /// Copies the selected rows or columns as TSV.
    ///
    /// Both `tabularText` and `string` are written for spreadsheet and text
    /// apps. Nothing is written when there is no selection, so the previous
    /// clipboard contents survive a failed copy.
    @discardableResult
    func copySelection(to pasteboard: NSPasteboard = .general) -> Bool {
        guard let text = document.copySelection() else { return false }
        pasteboard.declareTypes([.tabularText, .string], owner: nil)
        pasteboard.setString(text, forType: .tabularText)
        pasteboard.setString(text, forType: .string)
        return true
    }

    /// Pastes the pasteboard string as TSV.
    ///
    /// The document is called only when the pasteboard actually provides a
    /// string: an empty string is a valid one-cell paste, while a pasteboard
    /// without text must not insert an empty cell.
    @discardableResult
    func paste(from pasteboard: NSPasteboard = .general) -> Bool {
        guard isEditable,
              let type = pasteboard.availableType(from: [.string, .tabularText]),
              let text = pasteboard.string(forType: type) else {
            return false
        }
        document.pasteTSV(text)
        return true
    }

    /// Deletes the selected rows, or the selected columns.
    @discardableResult
    func deleteSelection() -> Bool {
        guard canDeleteSelection else { return false }
        document.deleteSelection()
        return true
    }

    /// Selects every row in the table.
    func selectAllRows() {
        guard let table = document.table, table.rowCount > 0 else { return }
        let rows = IndexSet(0..<table.rowCount)
        document.selection = GridSelection(rows: rows)

        guard let tableView else { return }
        isSyncingSelection = true
        tableView.selectRowIndexes(rows, byExtendingSelection: false)
        isSyncingSelection = false
        syncSelectionDisplay()
    }
}
