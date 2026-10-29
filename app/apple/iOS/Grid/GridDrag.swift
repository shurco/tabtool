import UIKit

extension GridController: UICollectionViewDelegate, UICollectionViewDragDelegate, UICollectionViewDropDelegate, UIGestureRecognizerDelegate {
    func installDrag(on view: UICollectionView) {
        view.delegate = self
        view.dragDelegate = self
        view.dropDelegate = self
        view.dragInteractionEnabled = true
        guard view.gestureRecognizers?.contains(where: { $0.name == "grid-column-pan" }) != true else { return }
        let pan = UIPanGestureRecognizer(target: self, action: #selector(reorderColumn(_:)))
        pan.name = "grid-column-pan"
        pan.delegate = self
        view.addGestureRecognizer(pan)
    }

    func collectionView(_ collectionView: UICollectionView,
                        contextMenuConfigurationForItemsAt indexPaths: [IndexPath],
                        point: CGPoint) -> UIContextMenuConfiguration? {
        let content = CGPoint(x: point.x + collectionView.contentOffset.x,
                              y: point.y + collectionView.contentOffset.y)
        let kind = elementKind(at: content, in: collectionView)
        if kind == GridLayout.columnHeader, let column = column(at: content, in: collectionView) {
            selectColumn(column)
        }
        return UIContextMenuConfiguration(identifier: nil, previewProvider: nil) { [weak self] _ in
            guard let self else { return UIMenu() }
            return kind == GridLayout.columnHeader ? self.headerMenu() : self.cellMenu()
        }
    }

    func collectionView(_ collectionView: UICollectionView,
                        itemsForBeginning session: UIDragSession,
                        at indexPath: IndexPath) -> [UIDragItem] {
        guard allowsStructuralEdits, document?.canEditRowStructure == true,
              let model = presenter?.modelRow(forDisplayRow: indexPath.section) else { return [] }
        let item = UIDragItem(itemProvider: NSItemProvider(object: "\(model)" as NSString))
        item.localObject = NSNumber(value: model)
        return [item]
    }

    func collectionView(_ collectionView: UICollectionView,
                        dropSessionDidUpdate session: UIDropSession,
                        withDestinationIndexPath destinationIndexPath: IndexPath?) -> UICollectionViewDropProposal {
        UICollectionViewDropProposal(operation: .move, intent: .insertAtDestinationIndexPath)
    }

    func collectionView(_ collectionView: UICollectionView,
                        performDropWith coordinator: UICollectionViewDropCoordinator) {
        guard let source = coordinator.items.first?.dragItem.localObject as? NSNumber else { return }
        let displayRow = coordinator.destinationIndexPath?.section ?? presenter?.numberOfRows ?? 0
        moveRow(modelRow: source.intValue, beforeDisplayRow: displayRow)
    }

    func gestureRecognizerShouldBegin(_ gestureRecognizer: UIGestureRecognizer) -> Bool {
        guard gestureRecognizer.name == "grid-column-pan",
              let pan = gestureRecognizer as? UIPanGestureRecognizer,
              let view = gestureRecognizer.view as? UICollectionView else { return true }
        let velocity = pan.velocity(in: view)
        guard allowsStructuralEdits, abs(velocity.x) > abs(velocity.y) else { return false }
        return column(at: contentPoint(pan.location(in: view), in: view), in: view) != nil
    }

    @objc func reorderColumn(_ pan: UIPanGestureRecognizer) {
        guard let view = pan.view as? UICollectionView else { return }
        let point = contentPoint(pan.location(in: view), in: view)
        switch pan.state {
        case .began:
            draggedColumn = column(at: point, in: view)
        case .ended:
            if let draggedColumn, let target = column(at: point, in: view) {
                moveColumn(from: draggedColumn, before: target)
            }
            draggedColumn = nil
        default:
            break
        }
    }

    func moveRow(modelRow: Int, beforeDisplayRow: Int) {
        guard allowsStructuralEdits, document?.canEditRowStructure == true else { return }
        let destination = presenter?.modelRow(forDisplayRow: beforeDisplayRow)
            ?? document?.table?.rowCount
            ?? beforeDisplayRow
        document?.moveRows(at: IndexSet(integer: modelRow), to: destination)
    }

    private func elementKind(at point: CGPoint, in view: UICollectionView) -> String? {
        guard let layout = view.collectionViewLayout as? GridLayout else { return nil }
        let hits = layout.layoutAttributesForElements(in: CGRect(origin: point, size: CGSize(width: 1, height: 1))) ?? []
        return hits.filter { $0.frame.contains(point) }.max { $0.zIndex < $1.zIndex }?.representedElementKind
    }

    private func column(at point: CGPoint, in view: UICollectionView) -> Int? {
        guard elementKind(at: point, in: view) == GridLayout.columnHeader,
              let layout = view.collectionViewLayout as? GridLayout else { return nil }
        let hits = layout.layoutAttributesForElements(in: CGRect(origin: point, size: CGSize(width: 1, height: 1))) ?? []
        return hits.first { $0.representedElementKind == GridLayout.columnHeader && $0.frame.contains(point) }?.indexPath.item
    }

    private func contentPoint(_ point: CGPoint, in view: UIScrollView) -> CGPoint {
        CGPoint(x: point.x + view.contentOffset.x, y: point.y + view.contentOffset.y)
    }
}
