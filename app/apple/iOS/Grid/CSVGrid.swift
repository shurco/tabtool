import SwiftUI
import UIKit

/// The editable CSV grid. The initializer matches the Mac grid so
/// `DocumentView` stays shared.
struct CSVGrid: UIViewRepresentable {
    @ObservedObject var document: CSVDocument
    let isEditable: Bool
    let allowsStructuralEdits: Bool
    @AppStorage(Preferences.Key.gridAlternatingRows) private var alternatingRows = Preferences.Default.alternatingRows
    @AppStorage(Preferences.Key.gridRowHeight) private var storedRowHeight = Preferences.Default.rowHeight
    @AppStorage(Preferences.Key.gridShowsRowNumbers) private var showsRowNumbers = Preferences.Default.showsRowNumbers

    func makeCoordinator() -> GridController { GridController() }

    func makeUIView(context: Context) -> UICollectionView {
        let view = GridCollectionView(frame: .zero, collectionViewLayout: GridLayout())
        view.controller = context.coordinator
        view.accessibilityIdentifier = "grid"
        view.accessibilityLabel = String(localized: "CSV Table")
        view.backgroundColor = .systemBackground
        view.dataSource = context.coordinator
        context.coordinator.collectionView = view
        context.coordinator.installGestures(on: view)
        view.register(GridDataCell.self, forCellWithReuseIdentifier: GridDataCell.reuseID)
        for kind in [GridLayout.columnHeader, GridLayout.rowNumber, GridLayout.corner] {
            view.register(GridChromeCell.self,
                          forSupplementaryViewOfKind: kind,
                          withReuseIdentifier: GridChromeCell.reuseID)
        }
        apply(context.coordinator, to: view)
        return view
    }

    func updateUIView(_ view: UICollectionView, context: Context) {
        apply(context.coordinator, to: view)
    }

    private func apply(_ controller: GridController, to view: UICollectionView) {
        controller.document = document
        controller.isEditable = isEditable
        controller.collectionView = view
        (view as? GridCollectionView)?.controller = controller
        controller.alternatingRows = alternatingRows
        controller.showsRowNumbers = showsRowNumbers
        controller.rowHeight = CGFloat(Preferences.validRowHeight(storedRowHeight))
        controller.apply(to: view)
    }
}
