import AppKit

/// Header view that owns explicit column selection.
///
/// AppKit does not support its column-selection mode in view-based tables, so
/// the header reports clicks to the coordinator and draws its own highlight.
final class CSVTableHeaderView: NSTableHeaderView {
    /// Distance from a column divider where a click starts a resize instead of
    /// selecting the column.
    private static let resizeMargin: CGFloat = 4

    var onColumnClick: ((Int, NSEvent.ModifierFlags) -> Void)?

    /// Builds the context menu for a column index.
    var menuProvider: ((Int) -> NSMenu?)?

    private(set) var selectedColumns = IndexSet() {
        didSet {
            if selectedColumns != oldValue {
                needsDisplay = true
            }
        }
    }

    /// Replaces the highlighted columns.
    func select(_ columns: IndexSet) {
        selectedColumns = columns
    }

    /// Reports a click inside `index`'s header to the selection handler.
    func clickColumn(at index: Int, modifiers: NSEvent.ModifierFlags) {
        guard index >= 0 else { return }
        onColumnClick?(index, modifiers)
    }

    /// The context menu for `index`, selecting the column first unless it is
    /// already part of the selection. A negative index has no menu.
    func contextMenu(forColumn index: Int) -> NSMenu? {
        guard index >= 0 else { return nil }
        if !selectedColumns.contains(index) {
            clickColumn(at: index, modifiers: [])
        }
        return menuProvider?(index)
    }

    override func menu(for event: NSEvent) -> NSMenu? {
        contextMenu(forColumn: column(at: convert(event.locationInWindow, from: nil)))
    }

    override func mouseDown(with event: NSEvent) {
        // Select before passing the event on: if this mouse-down starts a
        // column drag, the selection must be in place when the column-move
        // notification arrives, and the original index must not be reused
        // after AppKit has reordered the columns.
        let point = convert(event.locationInWindow, from: nil)
        if event.clickCount == 1, isSelectionClick(at: point) {
            clickColumn(at: self.column(at: point), modifiers: event.modifierFlags)
        }
        super.mouseDown(with: event)
    }

    override func draw(_ dirtyRect: NSRect) {
        super.draw(dirtyRect)
        guard !selectedColumns.isEmpty else { return }

        for column in selectedColumns {
            let rect = headerRect(ofColumn: column)
            guard rect.intersects(dirtyRect) else { continue }
            NSColor.selectedContentBackgroundColor.withAlphaComponent(0.18).setFill()
            rect.fill()
            NSColor.controlAccentColor.setFill()
            NSRect(x: rect.minX, y: rect.maxY - 2, width: rect.width, height: 2).fill()
        }
    }

    /// A click on a divider is a resize gesture, not a selection.
    private func isSelectionClick(at point: NSPoint) -> Bool {
        let column = self.column(at: point)
        guard column >= 0 else { return false }
        let rect = headerRect(ofColumn: column)
        return point.x > rect.minX + Self.resizeMargin
            && point.x < rect.maxX - Self.resizeMargin
    }
}
