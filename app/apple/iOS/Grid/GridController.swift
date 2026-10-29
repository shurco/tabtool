import SwiftUI
import UIKit

/// The collection view's data source. It projects the document through
/// `GridPresenter`, including a filter's display rows.
final class GridController: NSObject, UICollectionViewDataSource {
    var document: CSVDocument?
    var alternatingRows = false
    var showsRowNumbers = true
    var rowHeight: CGFloat = 24
    var isEditable = false
    var allowsStructuralEdits = false
    var focus: GridFocus?
    let editor = GridEditor()
    var draggedColumn: Int?
    weak var collectionView: UICollectionView?

    func apply(to collectionView: UICollectionView) {
        guard let layout = collectionView.collectionViewLayout as? GridLayout else { return }
        let rows = presenter?.numberOfRows ?? 0
        let columns = presenter?.numberOfColumns ?? 0
        validateFocus(rows: rows, columns: columns)
        layout.geometry = GridGeometry(rowHeight: rowHeight, showsRowNumbers: showsRowNumbers,
                                       rowCount: rows, columnCount: columns)
        layout.invalidateLayout()
        // ponytail: full reload. Update visible cells if a large edit stalls.
        collectionView.reloadData()
    }

    /// Drops a focus that the document no longer backs: a cleared selection or
    /// a cell outside the reparsed table would otherwise keep its ring and
    /// steer row-number and menu actions at a stale column.
    private func validateFocus(rows: Int, columns: Int) {
        guard let focus else { return }
        if (document?.selection.rows.isEmpty ?? true)
            || focus.displayRow < 0 || focus.displayRow >= rows
            || focus.column < 0 || focus.column >= columns {
            self.focus = nil
        }
    }

    func numberOfSections(in collectionView: UICollectionView) -> Int {
        presenter?.numberOfRows ?? 0
    }

    func collectionView(_ collectionView: UICollectionView, numberOfItemsInSection section: Int) -> Int {
        presenter?.numberOfColumns ?? 0
    }

    func collectionView(_ collectionView: UICollectionView,
                        cellForItemAt indexPath: IndexPath) -> UICollectionViewCell {
        let cell = collectionView.dequeueReusableCell(withReuseIdentifier: GridDataCell.reuseID, for: indexPath)
        guard let cell = cell as? GridDataCell, let presenter else { return cell }
        cell.configure(text: presenter.cell(row: indexPath.section, column: indexPath.item),
                       alignment: presenter.alignment(row: indexPath.section, column: indexPath.item),
                       row: indexPath.section,
                       alternating: alternatingRows,
                       rowHeight: rowHeight,
                       highlight: highlight(displayRow: indexPath.section, column: indexPath.item))
        return cell
    }

    func collectionView(_ collectionView: UICollectionView, viewForSupplementaryElementOfKind kind: String,
                        at indexPath: IndexPath) -> UICollectionReusableView {
        let view = collectionView.dequeueReusableSupplementaryView(
            ofKind: kind, withReuseIdentifier: GridChromeCell.reuseID, for: indexPath)
        guard let view = view as? GridChromeCell else { return view }
        switch kind {
        case GridLayout.columnHeader:
            view.configure(text: presenter?.header(at: indexPath.item) ?? "", centered: false,
                           highlighted: document?.selection.columns.contains(indexPath.item) == true)
        case GridLayout.rowNumber:
            let number = presenter?.modelRow(forDisplayRow: indexPath.section).map { String($0 + 1) } ?? ""
            let model = presenter?.modelRow(forDisplayRow: indexPath.section)
            let highlighted = model.map { document?.selection.rows.contains($0) == true } ?? false
            view.configure(text: number, centered: true, highlighted: highlighted)
        default:
            view.configure(text: "", centered: true)
        }
        return view
    }

    var presenter: GridPresenter? {
        guard let document, let source = document.rowSource else { return nil }
        return GridPresenter(table: source, configuration: document.configuration,
                             visibleRows: document.rowFilter.visibleRows)
    }
}
