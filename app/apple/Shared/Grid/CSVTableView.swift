import AppKit

/// The table view behind the grid.
///
/// It owns keyboard focus while no cell is being edited and starts editing the
/// selected cell on Return or Enter.
final class CSVTableView: NSTableView {
    /// The column the user last clicked. Row selection does not expose an
    /// active cell column, so Return editing falls back to this value.
    private(set) var lastClickedColumn = -1

    /// Clipboard, deletion, select-all, and edit validation live in the
    /// coordinator; the table view only routes responder actions to it.
    weak var coordinator: CSVGridCoordinator?

    override var acceptsFirstResponder: Bool {
        true
    }

    override func viewDidMoveToWindow() {
        super.viewDidMoveToWindow()
        guard let window else { return }
        if window.firstResponder === window || window.firstResponder == nil {
            window.makeFirstResponder(self)
        }
    }

    override func mouseDown(with event: NSEvent) {
        let point = convert(event.locationInWindow, from: nil)
        let clickedRow = row(at: point)
        let clickedColumn = column(at: point)
        if clickedColumn >= 0 {
            lastClickedColumn = clickedColumn
        }
        super.mouseDown(with: event)

        // The text field becomes editable only through the field editor, so a
        // double click starts editing explicitly.
        if event.clickCount == 2, editedRow < 0, clickedRow >= 0,
           clickedRow == selectedRow, clickedColumn >= 0,
           let delegate,
           delegate.tableView?(self, shouldEdit: tableColumns[clickedColumn],
                               row: clickedRow) == true {
            editColumn(clickedColumn, row: clickedRow, with: nil, select: true)
        }
    }

    override func keyDown(with event: NSEvent) {
        if isDeleteKey(event), coordinator?.deleteSelection() == true {
            return
        }
        if event.charactersIgnoringModifiers == "\r" || event.charactersIgnoringModifiers == "\u{3}",
           beginEditingSelectedCell() {
            return
        }
        super.keyDown(with: event)
    }

    // MARK: - Responder actions

    @objc func copy(_ sender: Any?) {
        coordinator?.copySelection()
    }

    @objc func paste(_ sender: Any?) {
        coordinator?.paste()
    }

    @objc func delete(_ sender: Any?) {
        coordinator?.deleteSelection()
    }

    override func selectAll(_ sender: Any?) {
        guard let coordinator else {
            super.selectAll(sender)
            return
        }
        coordinator.selectAllRows()
    }

    override func validateUserInterfaceItem(_ item: NSValidatedUserInterfaceItem) -> Bool {
        guard let coordinator else {
            return super.validateUserInterfaceItem(item)
        }
        switch item.action {
        case #selector(copy(_:)):
            return coordinator.canCopySelection
        case #selector(paste(_:)):
            return coordinator.canPaste
        case #selector(delete(_:)):
            return coordinator.canDeleteSelection
        case #selector(selectAll(_:)):
            return coordinator.canSelectAll
        default:
            return super.validateUserInterfaceItem(item)
        }
    }

    /// Delete, Backspace, and forward delete remove the selection.
    private func isDeleteKey(_ event: NSEvent) -> Bool {
        guard event.modifierFlags.intersection([.command, .control, .option]).isEmpty else {
            return false
        }
        switch event.charactersIgnoringModifiers {
        case "\u{7F}", "\u{08}", "\u{F728}":
            return true
        default:
            return false
        }
    }

    /// Starts editing the selected cell, returning `false` when editing is not
    /// allowed or there is no cell to edit.
    private func beginEditingSelectedCell() -> Bool {
        guard editedRow < 0, selectedRow >= 0, numberOfColumns > 0 else { return false }
        let column = preferredEditingColumn
        guard let delegate,
              delegate.tableView?(self, shouldEdit: tableColumns[column],
                                  row: selectedRow) == true else {
            return false
        }
        editColumn(column, row: selectedRow, with: nil, select: true)
        return true
    }

    private var preferredEditingColumn: Int {
        let candidate = selectedColumn >= 0 ? selectedColumn : lastClickedColumn
        return tableColumns.indices.contains(candidate) ? candidate : 0
    }
}
