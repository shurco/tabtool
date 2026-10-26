import AppKit

/// Bridges the document and the AppKit table view.
///
/// The coordinator owns no business state: it projects the document through
/// `GridPresenter`, commits edits straight into the document, and mirrors the
/// table selection into `document.selection`. Cell-only edits refresh visible
/// cell views in place so the field editor keeps focus; structural changes
/// reload the table and restore the selection.
///
/// View-based tables do not support AppKit's column-selection mode. The
/// coordinator therefore treats header clicks as explicit column selection and
/// draws that selection through `CSVTableHeaderView` and the cell views.
@MainActor
final class CSVGridCoordinator: NSObject, NSTableViewDataSource, NSTableViewDelegate,
                                NSTextFieldDelegate {
    private let document: CSVDocument
    private weak var tableView: CSVTableView?
    private var presenter: GridPresenter
    private var isEditable = false
    private var signature = TableSignature.empty
    private var isSyncingSelection = false
    private var displayedColumns = IndexSet()
    private var columnSelectionAnchor: Int?

    private var headerView: CSVTableHeaderView? {
        tableView?.headerView as? CSVTableHeaderView
    }

    init(document: CSVDocument) {
        self.document = document
        self.presenter = GridPresenter(table: document.table ?? .newDocument(),
                                       configuration: document.configuration)
        super.init()
    }

    /// Connects the coordinator to the table view built by the representable.
    func attach(to tableView: CSVTableView) {
        self.tableView = tableView
        tableView.dataSource = self
        tableView.delegate = self
        headerView?.onColumnClick = { [weak self] column, modifiers in
            self?.selectColumn(at: column, modifiers: modifiers)
        }
    }

    /// Applies the latest document snapshot to the table view.
    ///
    /// Columns are rebuilt only when their identity or count changes, and rows
    /// are reloaded only when their count changes. A cell edit updates the
    /// visible cell views without a reload, which preserves the field editor.
    func update(presenter: GridPresenter, isEditable: Bool) {
        guard let tableView else { return }
        self.presenter = presenter
        self.isEditable = isEditable

        let previous = signature
        let current = TableSignature(presenter: presenter)
        signature = current

        isSyncingSelection = true
        if previous.columnCount != current.columnCount || previous.columnIDs != current.columnIDs {
            rebuildColumns()
            tableView.reloadData()
        } else {
            if previous.headers != current.headers {
                updateHeaderTitles()
            }
            if previous.rowCount != current.rowCount {
                tableView.reloadData()
            } else {
                updateVisibleCells()
            }
        }
        isSyncingSelection = false

        if !selectionMatchesDocument {
            applySelectionFromDocument()
        }
        syncSelectionDisplay()
    }

    // MARK: - Data source

    func numberOfRows(in tableView: NSTableView) -> Int {
        presenter.numberOfRows
    }

    func tableView(_ tableView: NSTableView,
                   viewFor tableColumn: NSTableColumn?, row: Int) -> NSView? {
        guard let tableColumn else { return nil }
        let column = tableView.column(withIdentifier: tableColumn.identifier)
        guard column >= 0 else { return nil }

        let identifier = NSUserInterfaceItemIdentifier("grid-cell")
        let cell: CSVGridCellView
        if let reused = tableView.makeView(withIdentifier: identifier,
                                           owner: self) as? CSVGridCellView {
            cell = reused
        } else {
            cell = CSVGridCellView()
            cell.identifier = identifier
        }
        configure(cell, row: row, column: column)
        return cell
    }

    // MARK: - Delegate

    func tableView(_ tableView: NSTableView,
                   shouldEdit tableColumn: NSTableColumn?, row: Int) -> Bool {
        isEditable
    }

    func tableViewSelectionDidChange(_ notification: Notification) {
        guard !isSyncingSelection, let tableView else { return }
        let rows = tableView.selectedRowIndexes

        if rows.isEmpty {
            // Keep an explicit header selection when the user clicks outside
            // the rows; only clear when a row selection was actually removed.
            guard !document.selection.rows.isEmpty else { return }
            columnSelectionAnchor = nil
            document.selection = GridSelection()
        } else {
            columnSelectionAnchor = nil
            let selection = GridSelection(rows: rows)
            guard document.selection != selection else { return }
            document.selection = selection
        }
        syncSelectionDisplay()
    }

    func controlTextDidBeginEditing(_ notification: Notification) {
        // The document disables automatic undo grouping and registers explicit
        // groups per action. The native field editor would register its own
        // undo steps outside those groups, so it runs without undo support and
        // cell edits are undone through the document instead.
        if let fieldEditor = notification.userInfo?["NSFieldEditor"] as? NSTextView {
            fieldEditor.allowsUndo = false
        }
    }

    func controlTextDidEndEditing(_ notification: Notification) {
        guard let textField = notification.object as? NSTextField,
              let cell = textField.superview as? CSVGridCellView,
              let tableView,
              tableView.row(for: cell) == cell.row,
              tableView.column(for: cell) == cell.column else {
            return
        }
        document.setCell(textField.stringValue, row: cell.row, column: cell.column)
    }

    // MARK: - Columns and cells

    private func rebuildColumns() {
        guard let tableView else { return }
        for column in tableView.tableColumns {
            tableView.removeTableColumn(column)
        }
        for index in 0..<presenter.numberOfColumns {
            let column = NSTableColumn(identifier: NSUserInterfaceItemIdentifier("column-\(index)"))
            column.title = presenter.header(at: index)
            column.width = 100
            column.minWidth = 40
            column.resizingMask = .userResizingMask
            tableView.addTableColumn(column)
        }
    }

    private func updateHeaderTitles() {
        guard let tableView else { return }
        for (index, column) in tableView.tableColumns.enumerated() {
            column.title = presenter.header(at: index)
        }
    }

    private func updateVisibleCells() {
        guard let tableView else { return }
        let rows = tableView.rows(in: tableView.visibleRect)
        guard rows.length > 0 else { return }
        for row in rows.location..<(rows.location + rows.length) {
            guard row < presenter.numberOfRows else { break }
            for column in 0..<presenter.numberOfColumns {
                if row == tableView.editedRow && column == tableView.editedColumn {
                    continue
                }
                guard let cell = tableView.view(atColumn: column, row: row,
                                                makeIfNecessary: false) as? CSVGridCellView else {
                    continue
                }
                configure(cell, row: row, column: column)
            }
        }
    }

    private func configure(_ cell: CSVGridCellView, row: Int, column: Int) {
        cell.row = row
        cell.column = column
        cell.isColumnSelected = displayedColumns.contains(column)
        guard let textField = cell.textField else { return }
        textField.stringValue = presenter.cell(row: row, column: column)
        textField.alignment = presenter.alignment(row: row, column: column)
        textField.isEditable = isEditable
        textField.isSelectable = isEditable
        textField.delegate = self
    }

    // MARK: - Selection

    /// Handles an explicit header click.
    ///
    /// A plain click selects one column, Command-click toggles a column, and
    /// Shift-click extends the range from the last anchor. Selecting a column
    /// clears row selection; the document selection is either rows or columns.
    func selectColumn(at index: Int, modifiers: NSEvent.ModifierFlags) {
        guard let tableView, index >= 0, index < presenter.numberOfColumns else { return }

        var columns = document.selection.rows.isEmpty ? columnSelection : IndexSet()

        if modifiers.contains(.command) {
            if columns.contains(index) {
                columns.remove(index)
            } else {
                columns.insert(index)
            }
            if columns.isEmpty {
                columnSelectionAnchor = nil
            } else if columnSelectionAnchor == nil {
                columnSelectionAnchor = index
            }
        } else if modifiers.contains(.shift),
                  let anchor = columnSelectionAnchor ?? columns.min() {
            columns.formUnion(IndexSet(min(anchor, index)...max(anchor, index)))
        } else {
            columns = IndexSet(integer: index)
            columnSelectionAnchor = index
        }

        isSyncingSelection = true
        tableView.deselectAll(nil)
        isSyncingSelection = false

        document.selection = columns.isEmpty ? GridSelection() : GridSelection(columns: columns)
        syncSelectionDisplay()
    }

    /// The valid column selection the document currently describes.
    private var columnSelection: IndexSet {
        guard document.selection.rows.isEmpty else { return IndexSet() }
        return document.selection.columns.intersection(IndexSet(0..<presenter.numberOfColumns))
    }

    private var selectionMatchesDocument: Bool {
        guard let tableView else { return true }
        let rows = document.selection.rows.intersection(IndexSet(0..<presenter.numberOfRows))
        if !rows.isEmpty {
            return tableView.selectedRowIndexes == rows
        }
        return tableView.selectedRowIndexes.isEmpty
            && (headerView?.selectedColumns ?? IndexSet()) == columnSelection
    }

    private func applySelectionFromDocument() {
        guard let tableView else { return }
        let rows = document.selection.rows.intersection(IndexSet(0..<presenter.numberOfRows))

        isSyncingSelection = true
        if rows.isEmpty {
            tableView.deselectAll(nil)
        } else {
            tableView.selectRowIndexes(rows, byExtendingSelection: false)
        }
        isSyncingSelection = false

        columnSelectionAnchor = columnSelection.min()
        syncSelectionDisplay()
    }

    /// Shows the document's column selection in the header and cell views.
    private func syncSelectionDisplay() {
        let columns = columnSelection
        let changed = displayedColumns.symmetricDifference(columns)
        displayedColumns = columns
        headerView?.select(columns)

        guard let tableView, !changed.isEmpty, presenter.numberOfRows > 0 else { return }
        tableView.reloadData(forRowIndexes: IndexSet(0..<presenter.numberOfRows),
                             columnIndexes: changed)
    }
}
