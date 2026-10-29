import UIKit

/// Places one cell per display row and column, and pins the header row and
/// the row-number column to the visible rect.
final class GridLayout: UICollectionViewLayout {
    static let columnHeader = "column-header"
    static let rowNumber = "row-number"
    static let corner = "corner"

    var geometry = GridGeometry(rowHeight: 24, showsRowNumbers: true, rowCount: 0, columnCount: 0)

    override var collectionViewContentSize: CGSize { geometry.contentSize }

    override func shouldInvalidateLayout(forBoundsChange newBounds: CGRect) -> Bool { true }

    override func layoutAttributesForElements(in rect: CGRect) -> [UICollectionViewLayoutAttributes]? {
        guard geometry.columnCount > 0, geometry.rowHeight > 0 else { return [] }
        let visible = visibleRect
        var items: [UICollectionViewLayoutAttributes] = []
        for row in rows(intersecting: rect) {
            for column in columns(intersecting: rect) {
                let indexPath = IndexPath(item: column, section: row)
                if let item = layoutAttributesForItem(at: indexPath), item.frame.intersects(rect) {
                    items.append(item)
                }
            }
            if geometry.showsRowNumbers,
               let gutter = chromeAttributes(kind: Self.rowNumber, row: row, column: 0, visible: visible),
               gutter.frame.intersects(rect) {
                items.append(gutter)
            }
        }
        for column in columns(intersecting: rect.union(visible)) {
            if let header = chromeAttributes(kind: Self.columnHeader, row: 0, column: column, visible: visible),
               header.frame.intersects(rect) {
                items.append(header)
            }
        }
        if geometry.showsRowNumbers,
           let corner = chromeAttributes(kind: Self.corner, row: 0, column: 0, visible: visible),
           corner.frame.intersects(rect) {
            items.append(corner)
        }
        return items
    }

    override func layoutAttributesForItem(at indexPath: IndexPath) -> UICollectionViewLayoutAttributes? {
        let attributes = UICollectionViewLayoutAttributes(forCellWith: indexPath)
        attributes.frame = geometry.cellFrame(row: indexPath.section, column: indexPath.item)
        attributes.zIndex = 0
        return attributes
    }

    override func layoutAttributesForSupplementaryView(ofKind elementKind: String,
                                                       at indexPath: IndexPath) -> UICollectionViewLayoutAttributes? {
        chromeAttributes(kind: elementKind, row: indexPath.section, column: indexPath.item, visible: visibleRect)
    }

    private var visibleRect: CGRect {
        guard let collectionView else { return .zero }
        return CGRect(origin: collectionView.contentOffset, size: collectionView.bounds.size)
    }

    private func chromeAttributes(kind: String, row: Int, column: Int,
                                  visible: CGRect) -> UICollectionViewLayoutAttributes? {
        let indexPath = IndexPath(item: column, section: row)
        let attributes = UICollectionViewLayoutAttributes(forSupplementaryViewOfKind: kind, with: indexPath)
        switch kind {
        case Self.columnHeader:
            attributes.frame = geometry.pinnedHeaderFrame(column: column, visible: visible)
            attributes.zIndex = 2
        case Self.rowNumber:
            attributes.frame = geometry.pinnedGutterFrame(row: row, visible: visible)
            attributes.zIndex = 2
        case Self.corner:
            attributes.frame = geometry.pinnedCornerFrame(visible: visible)
            attributes.zIndex = 3
        default:
            return nil
        }
        return attributes
    }

    private func columns(intersecting rect: CGRect) -> Range<Int> {
        let width = GridGeometry.columnWidth
        let start = max(0, Int(floor((rect.minX - geometry.gutter) / width)))
        let end = min(geometry.columnCount, Int(ceil((rect.maxX - geometry.gutter) / width)))
        return start..<max(start, end)
    }

    private func rows(intersecting rect: CGRect) -> Range<Int> {
        let start = max(0, Int(floor((rect.minY - GridGeometry.headerHeight) / geometry.rowHeight)))
        let end = min(geometry.rowCount, Int(ceil((rect.maxY - GridGeometry.headerHeight) / geometry.rowHeight)))
        return start..<max(start, end)
    }
}
