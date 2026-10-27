import AppKit
import TabToolKit

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

    /// Save, close and quit handling of a large file's window; nil in memory.
    /// `nonisolated(unsafe)` so `responds(to:)` can read it; it is only set
    /// on the main actor.
    nonisolated(unsafe) private(set) var largeWindow: LargeDocumentWindow?

    override func viewDidMoveToWindow() {
        super.viewDidMoveToWindow()
        coordinator?.tableDidMoveToWindow(self)
        largeWindow?.detach()
        largeWindow = nil
        guard let window else { return }
        if let document = coordinator?.document, document.isLargeMode {
            largeWindow = LargeDocumentWindow(window: window, document: document, grid: self)
        }
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

    /// The context menu for `row`, selecting the row first unless it is already
    /// part of the selection. A negative row has no menu.
    func contextMenu(forRow row: Int) -> NSMenu? {
        guard row >= 0 else { return nil }
        if !selectedRowIndexes.contains(row) {
            selectRowIndexes(IndexSet(integer: row), byExtendingSelection: false)
        }
        return coordinator?.rowMenu()
    }

    override func menu(for event: NSEvent) -> NSMenu? {
        contextMenu(forRow: row(at: convert(event.locationInWindow, from: nil)))
    }

    override func keyDown(with event: NSEvent) {
        if isDeleteKey(event), coordinator?.deleteSelection() == true {
            return
        }
        if event.charactersIgnoringModifiers == "\r" || event.charactersIgnoringModifiers == "\u{3}",
           beginEditingSelectedCell() {
            return
        }
        if let row = edgeRow(for: event) {
            selectRowIndexes(IndexSet(integer: row), byExtendingSelection: false)
            scrollRowToVisible(row)
            return
        }
        super.keyDown(with: event)
    }

    /// The first row for Command-Up and the last for Command-Down; NSTableView
    /// itself treats them as plain arrows.
    private func edgeRow(for event: NSEvent) -> Int? {
        guard numberOfRows > 0,
              event.modifierFlags.intersection([.command, .shift, .option, .control]) == .command else {
            return nil
        }
        switch event.specialKey {
        case .upArrow: return 0
        case .downArrow: return numberOfRows - 1
        default: return nil
        }
    }

    // MARK: - Accessibility

    // An accessibility client that asks for every row makes AppKit create one
    // element per row: gigabytes for a large file. Large files expose only the
    // rows on screen.
    private var exposesOnlyVisibleRows: Bool {
        coordinator?.presenter.table is LargeCSVSource
    }

    @objc func accessibilityRows() -> [Any]? {
        if exposesOnlyVisibleRows { return visibleRowElements }
        return super.accessibilityRows()
    }

    override func accessibilityChildren() -> [Any]? {
        guard exposesOnlyVisibleRows else { return super.accessibilityChildren() }
        return visibleRowElements + (accessibilityColumns() ?? []) + [headerView].compactMap { $0 }
    }

    // Clients such as System Events and VoiceOver reach NSTableView through
    // the legacy attribute API, which bypasses the two overrides above. The
    // override is marked deprecated like the API it answers; AppKit calls it
    // on the main thread.
    @available(macOS, deprecated: 10.10)
    override func accessibilityAttributeValue(_ attribute: NSAccessibility.Attribute) -> Any? {
        nonisolated(unsafe) var elements: [Any]?
        MainActor.assumeIsolated {
            guard exposesOnlyVisibleRows else { return }
            switch attribute {
            case .children: elements = accessibilityChildren()
            case .rows: elements = visibleRowElements
            default: break
            }
        }
        return elements ?? super.accessibilityAttributeValue(attribute)
    }

    /// The on-screen row elements, read untyped: AppKit's elements do not all
    /// conform to the protocol the typed accessor promises.
    private var visibleRowElements: [Any] {
        let rows = perform(NSSelectorFromString("accessibilityVisibleRows"))?.takeUnretainedValue()
        return (rows as? NSArray).map { $0 as [AnyObject] } ?? []
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

    // Undo and redo go through the responder chain so a large file's own undo
    // stack (which is not the window's) is the one that answers the Edit menu.
    override var undoManager: UndoManager? {
        coordinator?.document.undoManager ?? super.undoManager
    }

    // A large file saves by streaming, never through the system document, so
    // its grid answers Save, Save As and Duplicate. In memory these actions
    // still reach the system document.
    nonisolated private static let largeFileActions: Set<Selector> = [
        #selector(NSDocument.save(_:)), #selector(NSDocument.saveAs(_:)), #selector(NSDocument.duplicate(_:)),
    ]

    override func responds(to selector: Selector!) -> Bool {
        if let selector, Self.largeFileActions.contains(selector) { return largeWindow != nil }
        return super.responds(to: selector)
    }

    @objc func saveDocument(_ sender: Any?) {
        largeWindow?.save()
    }

    @objc func saveDocumentAs(_ sender: Any?) {
        largeWindow?.saveAs()
    }

    /// Not available for large files; validation keeps the item disabled.
    @objc func duplicateDocument(_ sender: Any?) {}

    @objc func undo(_ sender: Any?) {
        undoManager?.undo()
    }

    @objc func redo(_ sender: Any?) {
        undoManager?.redo()
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
        case #selector(undo(_:)):
            return validate(item, canPerform: undoManager?.canUndo, title: undoManager?.undoMenuItemTitle)
                && coordinator.document.canEditCells
        case #selector(redo(_:)):
            return validate(item, canPerform: undoManager?.canRedo, title: undoManager?.redoMenuItemTitle)
                && coordinator.document.canEditCells
        case #selector(saveDocument(_:)), #selector(saveDocumentAs(_:)):
            return largeWindow?.canSave == true
        case #selector(duplicateDocument(_:)):
            return false
        default:
            return super.validateUserInterfaceItem(item)
        }
    }

    private func validate(_ item: NSValidatedUserInterfaceItem, canPerform: Bool?, title: String?) -> Bool {
        if let title, let menuItem = item as? NSMenuItem { menuItem.title = title }
        return canPerform == true
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
