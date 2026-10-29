import UIKit

extension GridController {
    func selectCell(displayRow: Int, column: Int) {
        guard let presenter,
              let model = presenter.modelRow(forDisplayRow: displayRow),
              (0..<presenter.numberOfColumns).contains(column) else { return }
        focus = GridFocus(displayRow: displayRow, column: column)
        document?.selection = GridSelection(rows: IndexSet(integer: model))
        collectionView?.reloadData()
    }

    func selectColumn(_ column: Int) {
        guard let presenter, (0..<presenter.numberOfColumns).contains(column) else { return }
        focus = nil
        document?.selection = GridSelection(columns: IndexSet(integer: column))
        collectionView?.reloadData()
    }

    /// Moves by one cell. A step past the last column wraps to the next row.
    func moveFocus(rowDelta: Int, columnDelta: Int) {
        guard let presenter, presenter.numberOfRows > 0, presenter.numberOfColumns > 0 else { return }
        let current = focus ?? GridFocus(displayRow: 0, column: 0)
        var row = current.displayRow + rowDelta
        var column = current.column + columnDelta
        while column >= presenter.numberOfColumns {
            column -= presenter.numberOfColumns
            row += 1
        }
        while column < 0 {
            column += presenter.numberOfColumns
            row -= 1
        }
        row = min(max(row, 0), presenter.numberOfRows - 1)
        column = min(max(column, 0), presenter.numberOfColumns - 1)
        selectCell(displayRow: row, column: column)
        collectionView?.scrollToItem(at: IndexPath(item: column, section: row),
                                     at: [.centeredHorizontally, .centeredVertically],
                                     animated: false)
    }

    func commitEditing(_ text: String) {
        guard isEditable, let focus, let model = presenter?.modelRow(forDisplayRow: focus.displayRow) else { return }
        document?.setCell(text, row: model, column: focus.column)
    }

    func highlight(displayRow: Int, column: Int) -> GridCellHighlight {
        if focus == GridFocus(displayRow: displayRow, column: column) { return .focused }
        if let model = presenter?.modelRow(forDisplayRow: displayRow),
           document?.selection.rows.contains(model) == true { return .selected }
        if document?.selection.columns.contains(column) == true { return .selected }
        return .none
    }

    func installGestures(on view: UICollectionView) {
        guard view.gestureRecognizers?.contains(where: { $0.name == "grid-tap" }) != true else { return }
        let single = UITapGestureRecognizer(target: self, action: #selector(handleTap(_:)))
        single.name = "grid-tap"
        view.addGestureRecognizer(single)
        view.allowsSelection = false
        installDrag(on: view)
    }

    func beginEditing() {
        guard isEditable, let focus, let collectionView else { return }
        let text = presenter?.cell(row: focus.displayRow, column: focus.column) ?? ""
        let indexPath = IndexPath(item: focus.column, section: focus.displayRow)
        guard let frame = collectionView.collectionViewLayout.layoutAttributesForItem(at: indexPath)?.frame else { return }
        editor.onCommit = { [weak self] text in self?.commitEditing(text) }
        editor.onPrevious = { [weak self] in self?.stepEditing(columnDelta: -1) }
        editor.onNext = { [weak self] in self?.stepEditing(columnDelta: 1) }
        editor.onDone = { [weak self] in self?.editor.dismiss(commit: true) }
        editor.show(in: collectionView, frame: frame, text: text)
    }

    private func stepEditing(columnDelta: Int) {
        editor.dismiss(commit: true)
        moveFocus(rowDelta: 0, columnDelta: columnDelta)
        beginEditing()
    }

    @objc func handleTap(_ gesture: UITapGestureRecognizer) {
        guard let view = gesture.view as? UICollectionView,
              let layout = view.collectionViewLayout as? GridLayout else { return }
        editor.dismiss(commit: true)
        view.becomeFirstResponder()
        let point = CGPoint(x: gesture.location(in: view).x + view.contentOffset.x,
                            y: gesture.location(in: view).y + view.contentOffset.y)
        let hits = layout.layoutAttributesForElements(in: CGRect(origin: point, size: CGSize(width: 1, height: 1))) ?? []
        guard let hit = hits.filter({ $0.frame.contains(point) }).max(by: { $0.zIndex < $1.zIndex }) else { return }
        let (element, tapped) = element(for: hit)
        switch GridTapResolution.resolve(element: element, focus: focus, tapped: tapped) {
        case .selectCell:
            selectCell(displayRow: hit.indexPath.section, column: hit.indexPath.item)
        case .editCell:
            beginEditing()
        case .selectColumn:
            selectColumn(hit.indexPath.item)
        case .selectRowNumber:
            selectCell(displayRow: hit.indexPath.section, column: focus?.column ?? 0)
        case .ignore:
            break
        }
    }

    private func element(for hit: UICollectionViewLayoutAttributes) -> (GridElementKind, GridFocus?) {
        if hit.representedElementCategory == .cell {
            return (.cell, GridFocus(displayRow: hit.indexPath.section, column: hit.indexPath.item))
        }
        if hit.representedElementKind == GridLayout.columnHeader { return (.columnHeader, nil) }
        if hit.representedElementKind == GridLayout.rowNumber { return (.rowNumber, nil) }
        return (.corner, nil)
    }
}

/// The text field shown over the focused cell.
final class GridEditor: NSObject, UITextFieldDelegate {
    let field = UITextField()
    var onCommit: (String) -> Void = { _ in }
    var onPrevious: () -> Void = {}
    var onNext: () -> Void = {}
    var onDone: () -> Void = {}

    func show(in collectionView: UICollectionView, frame: CGRect, text: String) {
        field.frame = frame.insetBy(dx: 1, dy: 1)
        field.text = text
        field.font = .systemFont(ofSize: 15)
        field.backgroundColor = .systemBackground
        field.borderStyle = .roundedRect
        field.autocapitalizationType = .none
        field.autocorrectionType = .no
        field.smartQuotesType = .no
        field.delegate = self
        field.inputAccessoryView = GridEditorAccessory.make(
            onPrevious: onPrevious, onNext: onNext, onDone: onDone)
        collectionView.addSubview(field)
        field.becomeFirstResponder()
    }

    func dismiss(commit: Bool) {
        guard field.superview != nil else { return }
        let text = field.text ?? ""
        field.resignFirstResponder()
        field.removeFromSuperview()
        if commit { onCommit(text) }
    }

    func textFieldShouldReturn(_ textField: UITextField) -> Bool {
        dismiss(commit: true)
        return false
    }
}
