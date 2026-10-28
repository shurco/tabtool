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
    @AppStorage(Preferences.Key.gridShowsRowNumbers) private var showsRowNumbers = Preferences.Default.showsRowNumbers

    /// A hand-edited stored height the settings do not offer must not reach the grid.
    private var rowHeight: CGFloat { CGFloat(Preferences.validRowHeight(storedRowHeight)) }

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
        tableView.gridStyleMask = .solidVerticalGridLineMask
        tableView.style = .fullWidth
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
        scrollView.verticalRulerView = RowNumberRuler(scrollView: scrollView, tableView: tableView)
        scrollView.hasVerticalRuler = true
        scrollView.rulersVisible = showsRowNumbers
        return scrollView
    }

    func updateNSView(_ scrollView: NSScrollView, context: Context) {
        guard let tableView = scrollView.documentView as? CSVTableView,
              let table = document.rowSource else {
            return
        }
        tableView.applyGridAppearance(alternatingRows: alternatingRows, rowHeight: rowHeight)
        context.coordinator.update(
            presenter: GridPresenter(table: table, configuration: document.configuration,
                                     visibleRows: document.rowFilter.visibleRows),
            isEditable: isEditable,
            allowsStructuralEdits: allowsStructuralEdits
        )
        if scrollView.rulersVisible != showsRowNumbers {
            scrollView.rulersVisible = showsRowNumbers
        }
        (scrollView.verticalRulerView as? RowNumberRuler)?.reload()
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
