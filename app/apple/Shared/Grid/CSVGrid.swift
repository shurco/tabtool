import AppKit
import SwiftUI

/// The editable CSV grid: a view-based `NSTableView` inside `NSScrollView`.
///
/// The document is the single source of truth; the coordinator projects it and
/// writes edits and selection back into it.
struct CSVGrid: NSViewRepresentable {
    @ObservedObject var document: CSVDocument
    let isEditable: Bool
    let allowsStructuralEdits: Bool
    @AppStorage(Preferences.Key.gridAlternatingRows) private var alternatingRows = Preferences.Default.alternatingRows
    @AppStorage(Preferences.Key.gridRowHeight) private var storedRowHeight = Preferences.Default.rowHeight

    /// The stored height when it is one the settings offer, so a hand-edited value cannot reach the grid.
    private var rowHeight: CGFloat {
        CGFloat(Preferences.rowHeights.contains(storedRowHeight) ? storedRowHeight : Preferences.Default.rowHeight)
    }

    func makeCoordinator() -> CSVGridCoordinator {
        CSVGridCoordinator(document: document)
    }

    func makeNSView(context: Context) -> NSScrollView {
        let tableView = CSVTableView()
        tableView.setAccessibilityIdentifier("grid")
        tableView.setAccessibilityLabel(String(localized: "CSV Table"))
        tableView.setAccessibilityRole(.table)
        tableView.setAccessibilityHelp(String(
            localized: "Use the arrow keys to move between cells. Press Return to edit the selected cell."
        ))
        tableView.refusesFirstResponder = false
        tableView.headerView = CSVTableHeaderView()
        tableView.columnAutoresizingStyle = .noColumnAutoresizing
        tableView.allowsMultipleSelection = true
        tableView.allowsColumnResizing = true
        tableView.allowsColumnReordering = true
        tableView.applyGridAppearance(alternatingRows: alternatingRows, rowHeight: rowHeight)
        tableView.intercellSpacing = NSSize(width: 8, height: 2)
        tableView.setDraggingSourceOperationMask(.move, forLocal: true)
        tableView.setDraggingSourceOperationMask(.copy, forLocal: false)
        tableView.registerForDraggedTypes([GridPasteboard.rowIndexes])
        tableView.draggingDestinationFeedbackStyle = .gap

        let scrollView = NSScrollView()
        scrollView.documentView = tableView
        scrollView.hasVerticalScroller = true
        scrollView.hasHorizontalScroller = true
        scrollView.autohidesScrollers = true

        context.coordinator.attach(to: tableView)
        return scrollView
    }

    func updateNSView(_ scrollView: NSScrollView, context: Context) {
        guard let tableView = scrollView.documentView as? CSVTableView,
              let table = document.rowSource else {
            return
        }
        tableView.applyGridAppearance(alternatingRows: alternatingRows, rowHeight: rowHeight)
        context.coordinator.update(
            presenter: GridPresenter(table: table, configuration: document.configuration),
            isEditable: isEditable,
            allowsStructuralEdits: allowsStructuralEdits
        )
    }
}

extension NSTableView {
    /// Applies the user's grid settings; unchanged values are skipped because
    /// this runs on every document update.
    func applyGridAppearance(alternatingRows: Bool, rowHeight: CGFloat) {
        if usesAlternatingRowBackgroundColors != alternatingRows {
            usesAlternatingRowBackgroundColors = alternatingRows
        }
        if self.rowHeight != rowHeight {
            self.rowHeight = rowHeight
        }
    }
}
