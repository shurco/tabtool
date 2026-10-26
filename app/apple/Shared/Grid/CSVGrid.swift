import AppKit
import SwiftUI

/// The editable CSV grid: a view-based `NSTableView` inside `NSScrollView`.
///
/// The document is the single source of truth; the coordinator projects it and
/// writes edits and selection back into it.
struct CSVGrid: NSViewRepresentable {
    @ObservedObject var document: CSVDocument
    let isEditable: Bool

    func makeCoordinator() -> CSVGridCoordinator {
        CSVGridCoordinator(document: document)
    }

    func makeNSView(context: Context) -> NSScrollView {
        let tableView = CSVTableView()
        tableView.setAccessibilityIdentifier("grid")
        tableView.headerView = NSTableHeaderView()
        tableView.columnAutoresizingStyle = .noColumnAutoresizing
        tableView.allowsMultipleSelection = true
        tableView.allowsColumnSelection = true
        tableView.allowsColumnResizing = true
        tableView.usesAlternatingRowBackgroundColors = true
        tableView.rowHeight = 24
        tableView.intercellSpacing = NSSize(width: 8, height: 2)

        let scrollView = NSScrollView()
        scrollView.documentView = tableView
        scrollView.hasVerticalScroller = true
        scrollView.hasHorizontalScroller = true
        scrollView.autohidesScrollers = true

        context.coordinator.attach(to: tableView)
        return scrollView
    }

    func updateNSView(_ scrollView: NSScrollView, context: Context) {
        guard scrollView.documentView is CSVTableView,
              let table = document.table else {
            return
        }
        context.coordinator.update(
            presenter: GridPresenter(table: table, configuration: document.configuration),
            isEditable: isEditable
        )
    }
}
