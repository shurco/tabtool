import AppKit
import Foundation

/// Bridges the document and the AppKit table view.
///
/// The coordinator owns no business state: it projects the document through
/// `GridPresenter`, commits edits straight into the document, and mirrors the
/// table selection into `document.selection`. Cell-only edits refresh visible
/// cell views in place so the field editor keeps focus; structural changes
/// reload the table and restore the selection.
@MainActor
final class CSVGridCoordinator: NSObject, NSTableViewDataSource, NSTableViewDelegate,
                                NSTextFieldDelegate {
    private let document: CSVDocument
    private weak var tableView: CSVTableView?
    private var presenter: GridPresenter
    private var isEditable = false
    private var signature = TableSignature.empty
    private var isSyncingSelection = false

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
        defer { isSyncingSelection = false }
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

        if document.selection != currentSelection {
            applySelectionFromDocument()
        }
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
        guard !isSyncingSelection else { return }
        let selection = currentSelection
        if document.selection != selection {
            document.selection = selection
        }
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
        guard let textField = cell.textField else { return }
        textField.stringValue = presenter.cell(row: row, column: column)
        textField.alignment = presenter.alignment(row: row, column: column)
        textField.isEditable = isEditable
        textField.isSelectable = isEditable
        textField.delegate = self
    }

    // MARK: - Selection

    /// The selection the table view currently shows, with rows taking
    /// precedence over columns.
    private var currentSelection: GridSelection {
        guard let tableView else { return GridSelection() }
        let rows = tableView.selectedRowIndexes
        if !rows.isEmpty {
            return GridSelection(rows: rows)
        }
        return GridSelection(columns: tableView.selectedColumnIndexes)
    }

    private func applySelectionFromDocument() {
        guard let tableView else { return }
        let rows = document.selection.rows.intersection(IndexSet(0..<presenter.numberOfRows))
        let columns = document.selection.columns
            .intersection(IndexSet(0..<presenter.numberOfColumns))
        guard rows != tableView.selectedRowIndexes
                || columns != tableView.selectedColumnIndexes else {
            return
        }

        tableView.deselectAll(nil)
        if !rows.isEmpty {
            tableView.selectRowIndexes(rows, byExtendingSelection: false)
        } else if !columns.isEmpty {
            tableView.selectColumnIndexes(columns, byExtendingSelection: false)
        }
    }
}

/// The table's shape and column identity, used to distinguish a cell edit from
/// a structural change.
private struct TableSignature: Equatable {
    let rowCount: Int
    let columnCount: Int
    let columnIDs: [UUID]
    let headers: [String]?

    static let empty = TableSignature(rowCount: 0, columnCount: 0,
                                      columnIDs: [], headers: nil)

    init(rowCount: Int, columnCount: Int, columnIDs: [UUID], headers: [String]?) {
        self.rowCount = rowCount
        self.columnCount = columnCount
        self.columnIDs = columnIDs
        self.headers = headers
    }

    init(presenter: GridPresenter) {
        self.init(rowCount: presenter.numberOfRows,
                  columnCount: presenter.numberOfColumns,
                  columnIDs: presenter.table.columns.map(\.id),
                  headers: presenter.table.headers)
    }
}

/// A table cell with a single, borderless text field.
@MainActor
final class CSVGridCellView: NSTableCellView {
    var row = 0
    var column = 0

    override init(frame frameRect: NSRect) {
        super.init(frame: frameRect)
        let textField = NSTextField()
        textField.translatesAutoresizingMaskIntoConstraints = false
        textField.isBordered = false
        textField.drawsBackground = false
        textField.usesSingleLineMode = true
        textField.lineBreakMode = .byTruncatingTail
        textField.focusRingType = .none
        addSubview(textField)
        self.textField = textField
        NSLayoutConstraint.activate([
            textField.leadingAnchor.constraint(equalTo: leadingAnchor, constant: 4),
            textField.trailingAnchor.constraint(equalTo: trailingAnchor, constant: -4),
            textField.centerYAnchor.constraint(equalTo: centerYAnchor),
        ])
    }

    required init?(coder: NSCoder) {
        nil
    }
}
