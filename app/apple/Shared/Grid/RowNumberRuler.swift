import AppKit

/// The row number gutter: a vertical ruler beside the grid that shows each
/// displayed row's 1-based model row number, so a filtered grid keeps the
/// numbers of the rows it shows.
///
/// A ruler rather than a table column keeps table column indexes equal to
/// model column indexes.
final class RowNumberRuler: NSRulerView {
    private static let font = NSFont.monospacedDigitSystemFont(ofSize: NSFont.smallSystemFontSize,
                                                               weight: .regular)
    private static let padding: CGFloat = 6

    private var tableView: NSTableView? { clientView as? NSTableView }

    init(scrollView: NSScrollView, tableView: NSTableView) {
        super.init(scrollView: scrollView, orientation: .verticalRuler)
        clientView = tableView
        // Since macOS 14 views do not clip by default, and the ruler chrome would
        // draw its separator over the toolbar and title bar.
        clipsToBounds = true
        // The floating header's blur sits above the ruler and would hide the corner.
        wantsLayer = true
        layer?.zPosition = 1
        reservedThicknessForMarkers = 0
        reservedThicknessForAccessoryView = 0
        reload()
    }

    required init(coder: NSCoder) {
        fatalError("init(coder:) is not supported")
    }

    /// The number shown beside display row `row`, or `nil` when there is no such row.
    func label(forRow row: Int) -> String? {
        guard let tableView, (0..<tableView.numberOfRows).contains(row),
              let presenter = (tableView.dataSource as? CSVGridCoordinator)?.presenter,
              let modelRow = presenter.modelRow(forDisplayRow: row) else {
            return nil
        }
        return String(modelRow + 1)
    }

    /// Fits the width to the largest shown number and redraws; call after the rows change.
    func reload() {
        let largest = label(forRow: (tableView?.numberOfRows ?? 0) - 1) ?? ""
        let digitWidth = ("0" as NSString).size(withAttributes: [.font: Self.font]).width
        let thickness = ceil(digitWidth * CGFloat(max(largest.count, 2))) + 2 * Self.padding
        if ruleThickness != thickness {
            ruleThickness = thickness
        }
        needsDisplay = true
    }

    /// Draws a plain gutter instead of the ruler chrome: the grid background, a
    /// trailing separator, and the header's bottom line carried over the corner.
    override func draw(_ dirtyRect: NSRect) {
        NSColor.controlBackgroundColor.setFill()
        bounds.fill()
        NSColor.separatorColor.setFill()
        NSRect(x: bounds.maxX - 1, y: bounds.minY, width: 1, height: bounds.height).fill()
        let corner = cornerRect
        if !corner.isEmpty {
            NSRect(x: bounds.minX, y: isFlipped ? corner.maxY - 1 : corner.minY,
                   width: bounds.width, height: 1).fill()
        }
        drawHashMarksAndLabels(in: dirtyRect)
    }

    override func drawHashMarksAndLabels(in rect: NSRect) {
        guard let tableView, let rows = Range(tableView.rows(in: tableView.visibleRect)) else { return }
        let rowsArea = bounds.divided(atDistance: cornerRect.height,
                                      from: isFlipped ? .minYEdge : .maxYEdge).remainder
        NSBezierPath.clip(rowsArea)
        let attributes: [NSAttributedString.Key: Any] = [
            .font: Self.font,
            .foregroundColor: NSColor.secondaryLabelColor,
        ]
        for row in rows {
            guard let label = label(forRow: row) as NSString? else { continue }
            let rowRect = convert(tableView.rect(ofRow: row), from: tableView)
            let size = label.size(withAttributes: attributes)
            label.draw(at: NSPoint(x: bounds.maxX - Self.padding - size.width,
                                   y: rowRect.midY - size.height / 2),
                       withAttributes: attributes)
        }
    }

    /// The part of the ruler beside the table header; empty when the header
    /// sits above the ruler rather than over the rows.
    private var cornerRect: NSRect {
        guard let headerView = tableView?.headerView else { return .zero }
        let header = convert(headerView.bounds, from: headerView)
        return bounds.intersection(NSRect(x: bounds.minX, y: header.minY,
                                          width: bounds.width, height: header.height))
    }
}
