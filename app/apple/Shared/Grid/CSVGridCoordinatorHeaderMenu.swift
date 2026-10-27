import AppKit
import TabToolKit

/// The column header's context menu and sort arrows.
extension CSVGridCoordinator {
    /// Builds the menu for a right click on `column`.
    func headerMenu(forColumn column: Int) -> NSMenu {
        let menu = NSMenu()
        menu.autoenablesItems = false

        let ascending = menuItem(String(localized: "Sort Ascending"),
                                 #selector(sortAscending(_:)), enabled: isEditable)
        let descending = menuItem(String(localized: "Sort Descending"),
                                  #selector(sortDescending(_:)), enabled: isEditable)
        ascending.representedObject = column
        descending.representedObject = column
        menu.addItem(ascending)
        menu.addItem(descending)
        menu.addItem(menuItem(String(localized: "Remove Sort"),
                              #selector(removeSort(_:)), enabled: document.canRemoveSort))
        menu.addItem(.separator())
        menu.addItem(menuItem(String(localized: "Move Column Left"),
                              #selector(moveColumnLeft(_:)),
                              enabled: document.canMoveSelectedColumns(by: -1)))
        menu.addItem(menuItem(String(localized: "Move Column Right"),
                              #selector(moveColumnRight(_:)),
                              enabled: document.canMoveSelectedColumns(by: 1)))
        return menu
    }

    /// Points the header's sort arrows at the sorted column.
    func syncSortArrows() {
        let sort = document.activeSort
        let column = presenter.table.columns.firstIndex { $0.id == sort?.columnID }
        headerView?.setSort(column: column, direction: sort?.direction)
    }

    private func menuItem(_ title: String, _ action: Selector, enabled: Bool) -> NSMenuItem {
        let item = NSMenuItem(title: title, action: action, keyEquivalent: "")
        item.target = self
        item.isEnabled = enabled
        return item
    }

    @objc private func sortAscending(_ sender: NSMenuItem) {
        sort(sender, direction: .ascending)
    }

    @objc private func sortDescending(_ sender: NSMenuItem) {
        sort(sender, direction: .descending)
    }

    private func sort(_ sender: NSMenuItem, direction: CSVSortDirection) {
        guard let column = sender.representedObject as? Int else { return }
        document.sortRows(column: column, direction: direction)
    }

    @objc private func removeSort(_ sender: NSMenuItem) {
        document.removeSort()
    }

    @objc private func moveColumnLeft(_ sender: NSMenuItem) {
        document.moveSelectedColumns(by: -1)
    }

    @objc private func moveColumnRight(_ sender: NSMenuItem) {
        document.moveSelectedColumns(by: 1)
    }
}
