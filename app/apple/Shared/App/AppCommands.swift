import SwiftUI
import TabToolKit

/// App-wide menu commands that act on the focused document.
struct AppCommands: Commands {
    @FocusedObject private var document: CSVDocument?
    @AppStorage(FormatBarVisibility.storageKey) private var showFormatBar = true
    @Environment(\.openWindow) private var openWindow

    var body: some Commands {
        CommandGroup(replacing: .appInfo) {
            Button("About TabTool") {
                openWindow(id: AboutView.windowID)
            }
        }

        CommandGroup(after: .saveItem) {
            Button("Revert to Saved") {
                guard let document else { return }
                RevertCoordinator.present(for: document)
            }
            .disabled(!(document?.canRevertToSaved ?? false))

            Button("Export…") {
                guard let document else { return }
                ExportCoordinator.present(for: document)
            }
            .keyboardShortcut("e", modifiers: [.command, .shift])
            .disabled(!isEditable)
        }

        CommandMenu("Table") {
            Button("Add Row Above") {
                document?.addRowAboveSelection()
            }
            .disabled(!isEditable)

            Button("Add Row Below") {
                document?.addRowBelowSelection()
            }
            .disabled(!isEditable)

            Divider()

            Button("Add Column Left") {
                document?.addColumnLeftOfSelection()
            }
            .disabled(!isEditable)

            Button("Add Column Right") {
                document?.addColumnRightOfSelection()
            }
            .disabled(!isEditable)

            Divider()

            Button("Delete Row(s)") {
                document?.deleteSelectedRows()
            }
            .disabled(!(document?.canDeleteSelectedRows ?? false))

            Button("Delete Column(s)") {
                document?.deleteSelectedColumns()
            }
            .disabled(!(document?.canDeleteSelectedColumns ?? false))

            Divider()

            Button("Sort Ascending") {
                sortSelectedColumn(.ascending)
            }
            .disabled(!(document?.canSortSelection ?? false))

            Button("Sort Descending") {
                sortSelectedColumn(.descending)
            }
            .disabled(!(document?.canSortSelection ?? false))

            Button("Remove Sort") {
                document?.removeSort()
            }
            .disabled(!(document?.canRemoveSort ?? false))

            Divider()

            Button("Move Row Up") {
                document?.moveSelectedRows(by: -1)
            }
            .keyboardShortcut(.upArrow, modifiers: [.command, .option])
            .disabled(!(document?.canMoveSelectedRows(by: -1) ?? false))

            Button("Move Row Down") {
                document?.moveSelectedRows(by: 1)
            }
            .keyboardShortcut(.downArrow, modifiers: [.command, .option])
            .disabled(!(document?.canMoveSelectedRows(by: 1) ?? false))

            Button("Move Column Left") {
                document?.moveSelectedColumns(by: -1)
            }
            .keyboardShortcut(.leftArrow, modifiers: [.command, .option])
            .disabled(!(document?.canMoveSelectedColumns(by: -1) ?? false))

            Button("Move Column Right") {
                document?.moveSelectedColumns(by: 1)
            }
            .keyboardShortcut(.rightArrow, modifiers: [.command, .option])
            .disabled(!(document?.canMoveSelectedColumns(by: 1) ?? false))
        }

        CommandGroup(after: .toolbar) {
            Toggle("Show Format Bar", isOn: $showFormatBar)
        }
    }

    private func sortSelectedColumn(_ direction: CSVSortDirection) {
        guard let document, let column = document.selection.columns.first else { return }
        document.sortRows(column: column, direction: direction)
    }

    private var isEditable: Bool {
        document?.isEditableContent ?? false
    }
}
