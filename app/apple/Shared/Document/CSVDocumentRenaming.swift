import Foundation
import TabToolKit

/// Column header renaming and its menu predicates.
///
/// Without a header, renaming turns Header on: the generated names (A, B, C…)
/// become the header and every row stays a data row.
extension CSVDocument {
    /// Renames the header of `column`; an empty name is allowed.
    ///
    /// Rows and column identities stay as they are, so the active sort is kept.
    func renameColumn(at column: Int, to name: String) {
        guard canRenameColumn(at: column), let table = tableAfterEndingEditing() else { return }

        var renamed = table
        guard renamed.renameHeader(name, at: column) else { return }
        var newConfiguration = configuration
        newConfiguration.firstRowAsHeader = true
        applyChange(table: renamed, configuration: newConfiguration,
                    includesUTF8BOM: hadUTF8BOM, actionName: "Rename Column",
                    sort: activeSort)
    }

    func canRenameColumn(at column: Int) -> Bool {
        guard isEditableContent, let table else { return false }
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
