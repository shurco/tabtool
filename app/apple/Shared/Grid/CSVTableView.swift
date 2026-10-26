import AppKit

/// The table view behind the grid.
///
/// It owns keyboard focus while no cell is being edited and starts editing the
/// selected cell on Return or Enter.
final class CSVTableView: NSTableView {
    /// The column the user last clicked. Row selection does not expose an
    /// active cell column, so Return editing falls back to this value.
    private(set) var lastClickedColumn = -1

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
        if event.charactersIgnoringModifiers == "\r" || event.charactersIgnoringModifiers == "\u{3}",
           beginEditingSelectedCell() {
            return
        }
        super.keyDown(with: event)
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
