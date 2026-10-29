import CoreGraphics

/// Cell frames for the iOS grid. The header row and the row-number column
/// stay pinned to the visible rect while the cells scroll underneath.
struct GridGeometry: Equatable {
    static let columnWidth: CGFloat = 120
    static let headerHeight: CGFloat = 28
    static let gutterWidth: CGFloat = 48

    let rowHeight: CGFloat
    let showsRowNumbers: Bool
    let rowCount: Int
    let columnCount: Int

    var gutter: CGFloat { showsRowNumbers ? Self.gutterWidth : 0 }

    var contentSize: CGSize {
        CGSize(width: gutter + CGFloat(columnCount) * Self.columnWidth,
               height: Self.headerHeight + CGFloat(rowCount) * rowHeight)
    }

    func cellFrame(row: Int, column: Int) -> CGRect {
        CGRect(x: gutter + CGFloat(column) * Self.columnWidth,
               y: Self.headerHeight + CGFloat(row) * rowHeight,
               width: Self.columnWidth,
               height: rowHeight)
    }

    func pinnedHeaderFrame(column: Int, visible: CGRect) -> CGRect {
        CGRect(x: gutter + CGFloat(column) * Self.columnWidth,
               y: visible.minY,
               width: Self.columnWidth,
               height: Self.headerHeight)
    }

    func pinnedGutterFrame(row: Int, visible: CGRect) -> CGRect {
        CGRect(x: visible.minX,
               y: Self.headerHeight + CGFloat(row) * rowHeight,
               width: gutter,
               height: rowHeight)
    }

    func pinnedCornerFrame(visible: CGRect) -> CGRect {
        CGRect(x: visible.minX, y: visible.minY, width: gutter, height: Self.headerHeight)
    }
}
