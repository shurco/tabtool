import AppKit
import SwiftUI
import TabToolKit
import UniformTypeIdentifiers

/// Presents the Export save panel and writes the exported data.
///
/// The table and document configuration are captured when the command runs,
/// so exporting never mutates the document and an in-flight panel cannot pick
/// up a half-applied edit. The write is atomic and only starts after every
/// validation path has passed.
@MainActor
final class ExportCoordinator {
    private let table: CSVTable
    private let documentConfiguration: CSVConfiguration
    private let baseName: String
    private let options: ExportOptions
    private var panel: NSSavePanel?

    init(table: CSVTable, documentConfiguration: CSVConfiguration,
         documentURL: URL?) {
        self.table = table
        self.documentConfiguration = documentConfiguration
        self.baseName = ExportPlan.baseName(forDocumentAt: documentURL)
        self.options = ExportOptions(configuration: documentConfiguration)
    }

    /// Shows the export panel for the focused document; a filtered document
    /// exports only the rows it shows.
    static func present(for document: CSVDocument) {
        guard let table = document.visibleTableAfterEndingEditing() else { return }
        let coordinator = ExportCoordinator(
            table: table,
            documentConfiguration: document.configuration,
            documentURL: document.fileURL
        )
        coordinator.run()
    }

    private func run() {
        let panel = NSSavePanel()
        panel.title = String(localized: "Export")
        panel.prompt = String(localized: "Export")
        panel.canCreateDirectories = true
        panel.isExtensionHidden = false
        panel.allowedContentTypes = [ExportPlan.contentType(for: options.configuration)]
        panel.nameFieldStringValue = ExportPlan.filename(
            baseName: baseName,
            configuration: options.configuration
        )
        panel.accessoryView = makeAccessoryView(for: panel)
        self.panel = panel

        if let window = NSApp.keyWindow {
            panel.beginSheetModal(for: window) { [self] response in
                finish(response: response)
            }
        } else {
            finish(response: panel.runModal())
        }
    }

    private func makeAccessoryView(for panel: NSSavePanel) -> NSView {
        let content = ExportOptionsView(options: options) { [weak self, weak panel] configuration in
            guard let self, let panel else { return }
            self.fileTypeDidChange(configuration, in: panel)
        }
        let hostingView = NSHostingView(rootView: content)
        hostingView.frame = NSRect(origin: .zero, size: hostingView.fittingSize)
        return hostingView
    }

    /// Keeps the panel's file type and extension in step with the separator.
    private func fileTypeDidChange(_ configuration: CSVConfiguration,
                                   in panel: NSSavePanel) {
        panel.allowedContentTypes = [ExportPlan.contentType(for: configuration)]
        panel.nameFieldStringValue = ExportPlan.renaming(
            panel.nameFieldStringValue,
            configuration: configuration
        )
    }

    private func finish(response: NSApplication.ModalResponse) {
        guard let panel else { return }
        guard response == .OK, let selectedURL = panel.url else {
            teardown()
            return
        }

        // The panel validated this exact URL and its replace confirmation.
        // Rewriting the extension here could overwrite a different file
        // without asking.
        let configuration = options.configuration
        let url = selectedURL
        teardown()

        switch ExportPlan.decision(
            table: table,
            documentConfiguration: documentConfiguration,
            exportConfiguration: configuration
        ) {
        case .proceed:
            write(to: url, configuration: configuration)
        case .confirmAmbiguous:
            confirmAmbiguousExport(to: url, configuration: configuration)
        case .blockedUnencodable:
            presentError(FormatErrorText.alertMessage(for: CSVError.unencodableText))
        }
    }

    private func confirmAmbiguousExport(to url: URL,
                                        configuration: CSVConfiguration) {
        let alert = NSAlert()
        alert.alertStyle = .warning
        alert.messageText = String(localized: "Export Anyway?")
        alert.informativeText = String(localized: "Quotes are off and a header or cell contains the separator or a line break. The exported file may not read back as the same table.")
        alert.addButton(withTitle: String(localized: "Export Anyway"))
        alert.addButton(withTitle: String(localized: "Cancel"))
        guard alert.runModal() == .alertFirstButtonReturn else { return }
        write(to: url, configuration: configuration, allowAmbiguousOutput: true)
    }

    private func write(to url: URL, configuration: CSVConfiguration,
                       allowAmbiguousOutput: Bool = false) {
        do {
            try ExportPlan.write(
                table: table,
                documentConfiguration: documentConfiguration,
                configuration: configuration,
                to: url,
                allowAmbiguousOutput: allowAmbiguousOutput
            )
            ReviewPrompt.recordEngagement()
        } catch let error as CSVError where error == .ambiguousOutput
            && !allowAmbiguousOutput {
            // Preflight and writer share the same input, so this only happens
            // if they drift. Ask rather than fail with a generic error.
            confirmAmbiguousExport(to: url, configuration: configuration)
        } catch {
            presentError(FormatErrorText.alertMessage(for: error))
        }
    }

    private func presentError(_ message: String) {
        let alert = NSAlert()
        alert.alertStyle = .warning
        alert.messageText = String(localized: "Export Not Completed")
        alert.informativeText = message
        alert.addButton(withTitle: String(localized: "OK"))
        alert.runModal()
    }

    private func teardown() {
        panel?.accessoryView = nil
        panel = nil
    }
}
