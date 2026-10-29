import UIKit
import TabToolKit

extension GridController {
    var focusedColumn: Int? {
        document?.selection.columns.first ?? focus?.column
    }

    func addRowAbove() { document?.addRowAboveSelection() }
    func addRowBelow() { document?.addRowBelowSelection() }
    func addColumnLeft() { insertColumn(offset: 0) }
    func addColumnRight() { insertColumn(offset: 1) }

    func deleteGridSelection() {
        guard allowsStructuralEdits else { return }
        document?.deleteSelection()
    }

    @discardableResult
    func copySelection() -> Bool {
        guard let text = document?.copySelection() else { return false }
        UIPasteboard.general.string = text
        return true
    }

    @discardableResult
    func paste() -> Bool {
        guard allowsStructuralEdits, let text = UIPasteboard.general.string else { return false }
        document?.pasteTSV(text)
        return true
    }

    func selectAll() {
        guard let presenter, presenter.numberOfRows > 0 else { return }
        let rows = presenter.modelRows(forDisplayRows: IndexSet(integersIn: 0..<presenter.numberOfRows))
        document?.selection = GridSelection(rows: rows)
        collectionView?.reloadData()
    }

    func sortFocusedColumn(_ direction: CSVSortDirection) {
        guard allowsStructuralEdits, let column = focusedColumn else { return }
        document?.sortRows(column: column, direction: direction)
    }

    func removeSort() {
        document?.removeSort()
    }

    func renameFocusedColumn(to name: String) {
        guard allowsStructuralEdits, let column = focusedColumn else { return }
        document?.renameColumn(at: column, to: name)
    }

    func moveRow(fromDisplayRow: Int, beforeDisplayRow: Int) {
        guard allowsStructuralEdits, document?.canEditRowStructure == true,
              let model = presenter?.modelRow(forDisplayRow: fromDisplayRow) else { return }
        let destination = presenter?.modelRow(forDisplayRow: beforeDisplayRow)
            ?? document?.table?.rowCount
            ?? beforeDisplayRow
        document?.moveRows(at: IndexSet(integer: model), to: destination)
    }

    func moveColumn(from column: Int, before destination: Int) {
        guard allowsStructuralEdits else { return }
        document?.moveColumns(at: IndexSet(integer: column), to: destination)
    }

    func beginRenamingFocusedColumn() {
        guard allowsStructuralEdits, let column = focusedColumn,
              document?.canRenameColumn(at: column) == true else { return }
        let alert = UIAlertController(title: String(localized: "Rename Column"), message: nil, preferredStyle: .alert)
        alert.addTextField { [weak self] field in
            field.text = self?.presenter?.header(at: column)
        }
        alert.addAction(UIAlertAction(title: String(localized: "Cancel"), style: .cancel))
        alert.addAction(UIAlertAction(title: String(localized: "Rename"), style: .default) { [weak self] _ in
            self?.renameFocusedColumn(to: alert.textFields?.first?.text ?? "")
        })
        present(alert)
    }

    func cellMenu() -> UIMenu {
        UIMenu(children: [
            action("Add Row Above", enabled: allowsStructuralEdits && document?.canInsertRows == true) { $0.addRowAbove() },
            action("Add Row Below", enabled: allowsStructuralEdits && document?.canInsertRows == true) { $0.addRowBelow() },
            action("Add Column Left", enabled: allowsStructuralEdits) { $0.addColumnLeft() },
            action("Add Column Right", enabled: allowsStructuralEdits) { $0.addColumnRight() },
            action("Delete", enabled: allowsStructuralEdits) { $0.deleteGridSelection() },
            action("Copy", enabled: document?.copySelection() != nil) { $0.copySelection() },
            action("Paste", enabled: allowsStructuralEdits && UIPasteboard.general.hasStrings) { $0.paste() },
            action("Select All", enabled: (presenter?.numberOfRows ?? 0) > 0) { $0.selectAll() }
        ])
    }

    func headerMenu() -> UIMenu {
        UIMenu(children: [
            action("Sort Ascending", enabled: allowsStructuralEdits) { $0.sortFocusedColumn(.ascending) },
            action("Sort Descending", enabled: allowsStructuralEdits) { $0.sortFocusedColumn(.descending) },
            action("Remove Sort", enabled: document?.canRemoveSort == true) { $0.removeSort() },
            action("Rename Column", enabled: allowsStructuralEdits && focusedColumn != nil) { $0.beginRenamingFocusedColumn() }
        ])
    }

    private func insertColumn(offset: Int) {
        guard allowsStructuralEdits, let column = focusedColumn else { return }
        document?.selection = GridSelection(columns: IndexSet(integer: column))
        if offset == 0 {
            document?.addColumnLeftOfSelection()
        } else {
            document?.addColumnRightOfSelection()
        }
    }

    private func action(_ title: String, enabled: Bool,
                        run: @escaping (GridController) -> Void) -> UIAction {
        UIAction(title: String(localized: String.LocalizationValue(title)),
                 attributes: enabled ? [] : .disabled) { [weak self] _ in
            guard let self else { return }
            run(self)
        }
    }

    private func present(_ alert: UIAlertController) {
        let scenes = UIApplication.shared.connectedScenes.compactMap { $0 as? UIWindowScene }
        guard let root = scenes.flatMap(\.windows).first(where: \.isKeyWindow)?.rootViewController
            ?? scenes.first?.windows.first?.rootViewController else { return }
        var presenter = root
        while let presented = presenter.presentedViewController {
            presenter = presented
        }
        presenter.present(alert, animated: true)
    }
}
