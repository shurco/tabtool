import AppKit

/// Clipboard, deletion, and select-all actions exposed by the grid.
///
/// The coordinator routes responder-chain actions here; the document remains
/// the only owner of the table and of the undo registration.
extension CSVGridCoordinator {
    /// Whether the current selection has rows or columns to copy.
    var canCopySelection: Bool {
        guard let source = document.rowSource else { return false }
        let rows = document.selection.rows.intersection(IndexSet(integersIn: 0..<source.rowCount))
        let columns = document.selection.columns
            .intersection(IndexSet(integersIn: 0..<source.columnCount))
        return !rows.isEmpty || !columns.isEmpty
    }

    /// Whether the pasteboard currently provides text for a paste.
    var canPaste: Bool {
        allowsRowStructuralEdits
            && NSPasteboard.general.availableType(from: [.string, .tabularText]) != nil
    }

    /// Whether the current selection can be deleted.
    var canDeleteSelection: Bool {
        allowsStructuralEdits && document.canDeleteSelection
    }

    /// Whether the grid shows rows to select.
    var canSelectAll: Bool {
        presenter.numberOfRows > 0
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
        guard allowsRowStructuralEdits,
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
        guard presenter.numberOfRows > 0 else { return }
        let displayRows = IndexSet(integersIn: 0..<presenter.numberOfRows)
        document.selection = GridSelection(rows: presenter.modelRows(forDisplayRows: displayRows))

        guard let tableView else { return }
        isSyncingSelection = true
        tableView.selectRowIndexes(displayRows, byExtendingSelection: false)
        isSyncingSelection = false
        syncSelectionDisplay()
    }
}
