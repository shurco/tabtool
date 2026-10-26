import AppKit
import SwiftUI
import TabToolKit
import UniformTypeIdentifiers

/// The file mapping and validation rules for an export.
///
/// These rules are independent of AppKit so they can be unit tested. The
/// export always follows the export configuration: a tab separator writes
/// TSV, every other separator writes CSV.
enum ExportPlan {
    /// The file name stem offered by the panel: the document name plus
    /// `-export`, or `Untitled-export` when the document has no file yet.
    static func baseName(forDocumentAt url: URL?) -> String {
        let name = url?.deletingPathExtension().lastPathComponent ?? ""
        return (name.isEmpty ? "Untitled" : name) + "-export"
    }

    /// The complete file name for the seeded configuration.
    static func filename(baseName: String, configuration: CSVConfiguration) -> String {
        "\(baseName).\(fileExtension(for: configuration))"
    }

    /// The file extension an export writes.
    static func fileExtension(for configuration: CSVConfiguration) -> String {
        configuration.columnSeparator == "\t" ? "tsv" : "csv"
    }

    /// The content type shown by the save panel.
    static func contentType(for configuration: CSVConfiguration) -> UTType {
        fileExtension(for: configuration) == "tsv"
            ? .tabSeparatedText
            : .commaSeparatedText
    }

    /// Replaces the extension of a file name while keeping its stem.
    static func renaming(_ filename: String, configuration: CSVConfiguration) -> String {
        let stem = (filename as NSString).deletingPathExtension
        guard !stem.isEmpty else { return filename }
        return "\(stem).\(fileExtension(for: configuration))"
    }

    /// The URL as written: its extension always matches the configuration.
    ///
    /// The save panel already validates the extension, but a typed name or a
    /// separator change can leave a stale one.
    static func outputURL(_ url: URL, configuration: CSVConfiguration) -> URL {
        let expected = fileExtension(for: configuration)
        guard url.pathExtension.lowercased() != expected else { return url }
        return url.deletingPathExtension().appendingPathExtension(expected)
    }

    /// The explicit header row an export writes.
    ///
    /// Mirrors `CSVExport`: an enabled header uses the document headers or
    /// generated column names. Validation runs against this row so it checks
    /// the same fields the writer will produce.
    static func header(table: CSVTable, configuration: CSVConfiguration) -> [String]? {
        guard configuration.firstRowAsHeader else { return nil }
        return table.headers ?? table.columns.indices.map {
            ColumnNameGenerator.name(for: $0)
        }
    }

    /// Writes the exported bytes to `url` atomically.
    ///
    /// Validation happens before any file access, so an unencodable or
    /// ambiguous export never leaves a partial file behind.
    static func write(table: CSVTable,
                      documentConfiguration: CSVConfiguration,
                      configuration: CSVConfiguration,
                      to url: URL,
                      allowAmbiguousOutput: Bool = false) throws {
        let data = try CSVExport.data(
            table: table,
            documentConfiguration: documentConfiguration,
            exportConfiguration: configuration,
            allowAmbiguousOutput: allowAmbiguousOutput
        )
        try data.write(to: url, options: .atomic)
    }

    /// What the UI does for a validation result.
    static func decision(for issue: CSVWriteIssue?) -> ExportDecision {
        switch issue {
        case nil:
            return .proceed
        case .ambiguousOutput:
            return .confirmAmbiguous
        case .unencodableText:
            return .blockedUnencodable
        }
    }
}

/// Whether export can write, needs confirmation, or cannot write at all.
enum ExportDecision: Equatable {
    case proceed
    case confirmAmbiguous
    case blockedUnencodable
}

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

    /// Shows the export panel for the focused document.
    static func present(for document: CSVDocument) {
        guard let table = document.table else { return }
        let coordinator = ExportCoordinator(
            table: table,
            documentConfiguration: document.configuration,
            documentURL: document.fileURL
        )
        coordinator.run()
    }

    private func run() {
        let panel = NSSavePanel()
        panel.title = "Export"
        panel.prompt = "Export"
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

        let configuration = options.configuration
        let url = ExportPlan.outputURL(selectedURL, configuration: configuration)
        teardown()

        switch ExportPlan.decision(for: issue(for: configuration)) {
        case .proceed:
            write(to: url, configuration: configuration)
        case .confirmAmbiguous:
            confirmAmbiguousExport(to: url, configuration: configuration)
        case .blockedUnencodable:
            presentError(FormatErrorText.alertMessage(for: CSVError.unencodableText))
        }
    }

    private func issue(for configuration: CSVConfiguration) -> CSVWriteIssue? {
        let header = ExportPlan.header(table: table, configuration: configuration)
        return CSVWriteValidation.issue(table: table, header: header,
                                        configuration: configuration)
    }

    private func confirmAmbiguousExport(to url: URL,
                                        configuration: CSVConfiguration) {
        let alert = NSAlert()
        alert.alertStyle = .warning
        alert.messageText = "Export Anyway?"
        alert.informativeText = "Quotes are off and a header or cell contains "
            + "the separator or a line break. The exported file may not read "
            + "back as the same table."
        alert.addButton(withTitle: "Export Anyway")
        alert.addButton(withTitle: "Cancel")
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
        } catch {
            presentError(FormatErrorText.alertMessage(for: error))
        }
    }

    private func presentError(_ message: String) {
        let alert = NSAlert()
        alert.alertStyle = .warning
        alert.messageText = "Export Not Completed"
        alert.informativeText = message
        alert.addButton(withTitle: "OK")
        alert.runModal()
    }

    private func teardown() {
        panel?.accessoryView = nil
        panel = nil
    }
}
