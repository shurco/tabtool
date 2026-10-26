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
/// Clipboard actions and drag/column reorder live in focused extensions.
@MainActor
final class CSVGridCoordinator: NSObject, NSTableViewDataSource, NSTableViewDelegate,
                                NSTextFieldDelegate {
    let document: CSVDocument
    private(set) weak var tableView: CSVTableView?
    private(set) var presenter: GridPresenter
    private(set) var isEditable = false
    private var signature = TableSignature.empty
    /// True while mirroring document selection into AppKit.
    var isSyncingSelection = false
    /// Columns currently highlighted in the header and cells.
    var displayedColumns = IndexSet()
    /// Anchor for Shift-click column ranges.
    var columnSelectionAnchor: Int?
    /// True while applying a column order, to ignore echoed notifications.
    var isSyncingColumns = false
    private var columnWidths: [UUID: CGFloat] = [:]

    private static let columnIdentifierPrefix = "column-"

    var headerView: CSVTableHeaderView? {
        tableView?.headerView as? CSVTableHeaderView
    }

    init(document: CSVDocument) {
        self.document = document
        self.presenter = GridPresenter(table: document.table ?? .newDocument(),
                                       configuration: document.configuration)
        super.init()
    }

    deinit {
        NotificationCenter.default.removeObserver(self)
    }

    /// Connects the coordinator to the table view built by the representable.
    func attach(to tableView: CSVTableView) {
        self.tableView = tableView
        tableView.coordinator = self
        tableView.dataSource = self
        tableView.delegate = self
        headerView?.onColumnClick = { [weak self] column, modifiers in
            self?.selectColumn(at: column, modifiers: modifiers)
        }
        NotificationCenter.default.addObserver(
            self, selector: #selector(columnDidMove(_:)),
            name: NSTableView.columnDidMoveNotification, object: tableView
        )
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
        captureColumnWidths(for: previous.columnIDs)
        signature = current

        isSyncingSelection = true
        if previous.columnCount != current.columnCount || previous.columnIDs != current.columnIDs {
            rebuildColumns()
            let validIDs = Set(current.columnIDs)
            columnWidths = columnWidths.filter { validIDs.contains($0.key) }
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
        isSyncingColumns = true
        defer { isSyncingColumns = false }

        for column in tableView.tableColumns {
            tableView.removeTableColumn(column)
        }
        for index in 0..<presenter.numberOfColumns {
            let column = NSTableColumn(identifier: columnIdentifier(at: index))
            column.title = presenter.header(at: index)
            column.width = columnWidths[presenter.table.columns[index].id] ?? 100
            column.minWidth = 40
            column.resizingMask = .userResizingMask
            tableView.addTableColumn(column)
        }
    }

    /// Remembers the width of every current table column by model column ID.
    ///
    /// Identifiers encode the index the column had at the last rebuild, which
    /// is also its index in `columnIDs`; this survives a user column move and
    /// lets a rebuild restore the width of each model column.
    private func captureColumnWidths(for columnIDs: [UUID]) {
        guard let tableView else { return }
        for tableColumn in tableView.tableColumns {
            guard let index = columnIndex(of: tableColumn.identifier),
                  columnIDs.indices.contains(index) else {
                continue
            }
            columnWidths[columnIDs[index]] = tableColumn.width
        }
    }

    private func columnIdentifier(at index: Int) -> NSUserInterfaceItemIdentifier {
        NSUserInterfaceItemIdentifier("\(Self.columnIdentifierPrefix)\(index)")
    }

    private func columnIndex(of identifier: NSUserInterfaceItemIdentifier) -> Int? {
        guard identifier.rawValue.hasPrefix(Self.columnIdentifierPrefix) else {
            return nil
        }
        return Int(identifier.rawValue.dropFirst(Self.columnIdentifierPrefix.count))
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
}
