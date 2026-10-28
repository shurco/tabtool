import AppKit

/// A grid row whose background, alternating or not, keeps the last column's
/// trailing border visible.
final class CSVTableRowView: NSTableRowView {
    override func drawBackground(in dirtyRect: NSRect) {
        super.drawBackground(in: dirtyRect)
        (superview as? CSVTableView)?.drawTrailingGridLine(in: self, clipRect: dirtyRect)
    }
}
