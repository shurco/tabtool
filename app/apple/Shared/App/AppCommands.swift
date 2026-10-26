import SwiftUI

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
        }

        CommandGroup(after: .toolbar) {
            Toggle("Show Format Bar", isOn: $showFormatBar)
        }

        CommandGroup(replacing: .help) {
            Link("Help", destination: AboutView.repositoryURL)
            Button("TabTool Notice") {
                openWindow(id: AboutView.windowID)
            }
        }
    }

    private var isEditable: Bool {
        document?.isEditableContent ?? false
    }
}
