import AppKit

/// The data rows' context menu.
extension CSVGridCoordinator {
    /// Builds the menu for a right click on a row.
    func rowMenu() -> NSMenu {
        let menu = NSMenu()
        menu.autoenablesItems = false
        menu.addItem(menuItem(String(localized: "Add Row Above"),
                              #selector(addRowAbove(_:)), enabled: allowsRowStructuralEdits))
        menu.addItem(menuItem(String(localized: "Add Row Below"),
                              #selector(addRowBelow(_:)), enabled: allowsRowStructuralEdits))
        return menu
    }

    @objc private func addRowAbove(_ sender: NSMenuItem) {
        document.addRowAboveSelection()
    }

    @objc private func addRowBelow(_ sender: NSMenuItem) {
        document.addRowBelowSelection()
    }
}
