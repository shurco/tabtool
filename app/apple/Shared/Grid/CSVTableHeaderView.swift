import AppKit
import TabToolKit

/// Header view that owns explicit column selection.
///
/// AppKit does not support its column-selection mode in view-based tables, so
/// the header reports clicks to the coordinator and draws its own highlight
/// and the sort arrows at the right edge of every column.
final class CSVTableHeaderView: NSTableHeaderView {
    /// Distance from a column divider where a click starts a resize instead of
    /// selecting the column.
    private static let resizeMargin: CGFloat = 4

    /// Width of the sort arrow hit area at a column's right edge.
    private static let sortArrowWidth: CGFloat = 20

    var onColumnClick: ((Int, NSEvent.ModifierFlags) -> Void)?

    /// Reports a click on a column's sort arrow.
    var onSortClick: ((Int) -> Void)?

    /// Builds the context menu for a column index.
    var menuProvider: ((Int) -> NSMenu?)?

    private(set) var selectedColumns = IndexSet() {
        didSet {
            if selectedColumns != oldValue {
                needsDisplay = true
            }
        }
    }

    /// The sorted column and its direction; `nil` when no sort is active.
    private(set) var sort: (column: Int, direction: CSVSortDirection)? {
        didSet {
            if sort?.column != oldValue?.column || sort?.direction != oldValue?.direction {
                needsDisplay = true
            }
        }
    }

    /// Replaces the sorted column; repaints only when it changed.
    func setSort(column: Int?, direction: CSVSortDirection?) {
        sort = column.flatMap { column in direction.map { (column, $0) } }
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
        if event.clickCount == 1, let column = sortArrowColumn(at: point) {
            onSortClick?(column)
            return
        }
        if event.clickCount == 1, isSelectionClick(at: point) {
            clickColumn(at: self.column(at: point), modifiers: event.modifierFlags)
        }
        super.mouseDown(with: event)
    }

    /// The column whose sort arrow contains `point`, if any.
    func sortArrowColumn(at point: NSPoint) -> Int? {
        let column = self.column(at: point)
        guard column >= 0 else { return nil }
        let rect = headerRect(ofColumn: column)
        return point.x >= rect.maxX - Self.sortArrowWidth
            && point.x < rect.maxX - Self.resizeMargin ? column : nil
    }

    override func draw(_ dirtyRect: NSRect) {
        super.draw(dirtyRect)

        for column in selectedColumns {
            let rect = headerRect(ofColumn: column)
            guard rect.intersects(dirtyRect) else { continue }
            NSColor.selectedContentBackgroundColor.withAlphaComponent(0.18).setFill()
            rect.fill()
            NSColor.controlAccentColor.setFill()
            NSRect(x: rect.minX, y: rect.maxY - 2, width: rect.width, height: 2).fill()
        }

        for column in 0..<(tableView?.numberOfColumns ?? 0) {
            let rect = headerRect(ofColumn: column)
            guard rect.intersects(dirtyRect) else { continue }
            drawSortArrows(in: rect, active: sort?.column == column ? sort?.direction : nil)
        }
    }

    /// Draws the up/down arrow pair; only the active direction uses the accent colour.
    private func drawSortArrows(in rect: NSRect, active: CSVSortDirection?) {
        let half: CGFloat = 4
        let gap: CGFloat = 1.5
        let upward: CGFloat = isFlipped ? -1 : 1
        let center = NSPoint(x: rect.maxX - Self.sortArrowWidth / 2 - 2, y: rect.midY)
        for (direction, side) in [(CSVSortDirection.ascending, upward), (.descending, -upward)] {
            let base = center.y + side * gap
            let path = NSBezierPath()
            path.move(to: NSPoint(x: center.x - half, y: base))
            path.line(to: NSPoint(x: center.x + half, y: base))
            path.line(to: NSPoint(x: center.x, y: base + side * half))
            path.close()
            (active == direction ? NSColor.controlAccentColor : NSColor.tertiaryLabelColor).setFill()
            path.fill()
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
