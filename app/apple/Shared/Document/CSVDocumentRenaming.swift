import Foundation
import TabToolKit

/// Column header renaming and its menu predicates.
///
/// Only a table whose first row is a header has names to change; without one,
/// column names are generated (A, B, C…) and are not part of the file.
extension CSVDocument {
    /// Renames the header of `column`; an empty name is allowed.
    ///
    /// Rows and column identities stay as they are, so the active sort is kept.
    func renameColumn(at column: Int, to name: String) {
        guard canRenameColumn(at: column), let table = tableAfterEndingEditing() else { return }

        var renamed = table
        guard renamed.renameHeader(name, at: column) else { return }
        applyChange(table: renamed, configuration: configuration,
                    includesUTF8BOM: hadUTF8BOM, actionName: "Rename Column",
                    sort: activeSort)
    }

    func canRenameColumn(at column: Int) -> Bool {
        guard isEditableContent, let table, table.headers != nil else { return false }
        return table.columns.indices.contains(column)
    }

    /// The only selected column, when it can be renamed.
    var selectedColumnToRename: Int? {
        guard selection.columns.count == 1, let column = selection.columns.first,
              canRenameColumn(at: column) else { return nil }
        return column
    }

    var canRenameSelectedColumn: Bool {
        selectedColumnToRename != nil
    }

    /// Opens the inline header editor on the selected column.
    func renameSelectedColumn() {
        guard let column = selectedColumnToRename else { return }
        editingSession?.beginRenamingColumn(at: column)
    }
}
