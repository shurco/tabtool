import AppKit

/// Internal row drag and header-driven column reorder.
///
/// Row drags carry the row indexes in a private pasteboard type and TSV for
/// external apps. Only drops from the same table are accepted. Column moves
/// arrive as `NSTableView` notifications and are mapped onto the document.
extension CSVGridCoordinator {
    // MARK: - Row drag

    /// Provides one pasteboard item per dragged row.
    ///
    /// Every item carries the full dragged row set in a private type for an
    /// internal move. Only the first selected row also carries the TSV block,
    /// so external apps see one text item instead of concatenating per-row
    /// items without separators.
    func tableView(_ tableView: NSTableView,
                   pasteboardWriterForRow row: Int) -> NSPasteboardWriting? {
        guard isEditable, let table = document.table,
              (0..<table.rowCount).contains(row) else {
            return nil
        }
        let rows = document.selection.rows.contains(row)
            ? document.selection.rows : IndexSet(integer: row)
        let item = NSPasteboardItem()
        item.setString(rows.map(String.init).joined(separator: ","),
                       forType: GridPasteboard.rowIndexes)
        if row == rows.first, let tsv = document.copyRows(rows) {
            item.setString(tsv, forType: .tabularText)
            item.setString(tsv, forType: .string)
        }
        return item
    }

    func tableView(_ tableView: NSTableView, validateDrop info: NSDraggingInfo,
                   proposedRow row: Int,
                   proposedDropOperation dropOperation: NSTableView.DropOperation)
    -> NSDragOperation {
        self.dropOperation(proposed: dropOperation, source: info.draggingSource,
                           pasteboard: info.draggingPasteboard)
    }

    func tableView(_ tableView: NSTableView, acceptDrop info: NSDraggingInfo,
                   row: Int, dropOperation: NSTableView.DropOperation) -> Bool {
        guard self.dropOperation(proposed: dropOperation, source: info.draggingSource,
                                 pasteboard: info.draggingPasteboard) == .move else {
            return false
        }
        return moveRowsForDrop(rowIndexes(from: info.draggingPasteboard), to: row)
    }

    /// Validates a row drop: only this table's own drags, only between rows.
    func dropOperation(proposed: NSTableView.DropOperation, source: Any?,
                       pasteboard: NSPasteboard) -> NSDragOperation {
        guard isEditable,
              proposed == .above,
              let tableView,
              let source = source as? CSVTableView,
              source === tableView,
              pasteboard.availableType(from: [GridPasteboard.rowIndexes]) != nil else {
            return []
        }
        return .move
    }

    /// The row indexes carried by an internal row drag.
    func rowIndexes(from pasteboard: NSPasteboard) -> IndexSet {
        var rows = IndexSet()
        for item in pasteboard.pasteboardItems ?? [] {
            guard let text = item.string(forType: GridPasteboard.rowIndexes) else {
                continue
            }
            for token in text.split(separator: ",") {
                if let row = Int(token), row >= 0 {
                    rows.insert(row)
                }
            }
        }
        return rows
    }

    /// Moves the dragged rows and selects them at their new position.
    @discardableResult
    func moveRowsForDrop(_ indexes: IndexSet, to destination: Int) -> Bool {
        guard isEditable, let table = document.table else { return false }
        let selected = indexes.intersection(IndexSet(0..<table.rowCount))
        guard !selected.isEmpty else { return false }

        let insertion = min(max(destination, 0), table.rowCount)
        let movedInsertion = insertion - selected.filter { $0 < insertion }.count
        document.moveRows(at: selected, to: insertion)
        document.selection = GridSelection(
            rows: IndexSet(movedInsertion..<(movedInsertion + selected.count))
        )
        return true
    }

    // MARK: - Column reorder

    /// Applies an `NSTableView` header move to the document.
    ///
    /// Rebuilding columns and updating the document both run on the main
    /// actor; the guard stops a programmatic rebuild from being interpreted as
    /// another user move.
    @objc func columnDidMove(_ notification: Notification) {
        guard !isSyncingColumns,
              let tableView,
              notification.object as? NSTableView === tableView,
              let oldIndex = (notification.userInfo?["NSOldColumn"] as? NSNumber)?.intValue,
              let newIndex = (notification.userInfo?["NSNewColumn"] as? NSNumber)?.intValue,
              let table = document.table,
              table.columns.indices.contains(oldIndex),
              table.columns.indices.contains(newIndex),
              oldIndex != newIndex else {
            return
        }

        let selected = document.selection.columns
        isSyncingColumns = true
        defer { isSyncingColumns = false }
        document.moveColumn(from: oldIndex, to: newIndex)
        if !selected.isEmpty {
            document.selection = GridSelection(
                rows: document.selection.rows,
                columns: remappedColumns(selected, from: oldIndex, to: newIndex)
            )
        }
    }

    /// Maps selected columns through a remove-and-insert column move.
    private func remappedColumns(_ columns: IndexSet, from source: Int,
                                 to destination: Int) -> IndexSet {
        var remapped = IndexSet()
        for index in columns {
            if index == source {
                remapped.insert(destination)
            } else {
                var adjusted = index - (source < index ? 1 : 0)
                if adjusted >= destination {
                    adjusted += 1
                }
                remapped.insert(adjusted)
            }
        }
        return remapped
    }
}

/// Pasteboard types owned by the grid.
enum GridPasteboard {
    /// App-internal row indexes of one row drag, comma separated.
    static let rowIndexes = NSPasteboard.PasteboardType("app.shurco.tabtool.grid-rows")
}
