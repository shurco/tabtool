import SwiftUI
import TabToolKit
import UIKit

/// Presents export options, then the system document picker.
@MainActor
final class ExportCoordinator {
    private static var sheet: UIViewController?
    private static var cleanups: [ExportFileCleanup] = []

    /// Shows the export sheet for the focused document. A filtered document
    /// exports only the rows it shows.
    static func present(for document: CSVDocument) {
        guard let table = document.visibleTableAfterEndingEditing() else { return }
        let documentConfiguration = document.configuration
        let baseName = ExportPlan.baseName(forDocumentAt: document.fileURL)
        let options = ExportOptions(configuration: documentConfiguration)
        let root = ExportSheet(options: options, onCancel: {
            dismissSheet()
        }, onExport: { configuration in
            dismissSheet {
                export(table: table, documentConfiguration: documentConfiguration,
                       configuration: configuration, baseName: baseName)
            }
        })
        let host = UIHostingController(rootView: root)
        host.modalPresentationStyle = .formSheet
        sheet = host
        if !KeyWindow.present(host) {
            sheet = nil
        }
    }

    /// Writes an export to a temporary file the document picker can copy.
    static func prepare(table: CSVTable, documentConfiguration: CSVConfiguration,
                        configuration: CSVConfiguration, baseName: String,
                        allowAmbiguousOutput: Bool = false) throws -> URL {
        let directory = FileManager.default.temporaryDirectory
            .appendingPathComponent(UUID().uuidString, isDirectory: true)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        let url = directory.appendingPathComponent(
            ExportPlan.filename(baseName: baseName, configuration: configuration))
        do {
            try ExportPlan.write(table: table, documentConfiguration: documentConfiguration,
                                 configuration: configuration, to: url,
                                 allowAmbiguousOutput: allowAmbiguousOutput)
        } catch {
            removeTemporaryExport(url)
            throw error
        }
        return url
    }

    /// Deletes the export's temporary directory. A file that already sits in
    /// the system temporary directory is left alone.
    static func removeTemporaryExport(_ file: URL) {
        let directory = file.deletingLastPathComponent().standardizedFileURL
        guard directory != FileManager.default.temporaryDirectory.standardizedFileURL else { return }
        try? FileManager.default.removeItem(at: directory)
    }

    private static func export(table: CSVTable, documentConfiguration: CSVConfiguration,
                               configuration: CSVConfiguration, baseName: String) {
        switch ExportPlan.decision(table: table, documentConfiguration: documentConfiguration,
                                   exportConfiguration: configuration) {
        case .proceed:
            write(table: table, documentConfiguration: documentConfiguration,
                  configuration: configuration, baseName: baseName,
                  allowAmbiguousOutput: false)
        case .confirmAmbiguous:
            confirmAmbiguous(table: table, documentConfiguration: documentConfiguration,
                             configuration: configuration, baseName: baseName)
        case .blockedUnencodable:
            presentError(FormatErrorText.alertMessage(for: CSVError.unencodableText))
        }
    }

    private static func confirmAmbiguous(table: CSVTable, documentConfiguration: CSVConfiguration,
                                         configuration: CSVConfiguration, baseName: String) {
        let alert = UIAlertController(
            title: String(localized: "Export Anyway?"),
            message: String(localized: "Quotes are off and a header or cell contains the separator or a line break. The exported file may not read back as the same table."),
            preferredStyle: .alert)
        alert.addAction(UIAlertAction(title: String(localized: "Export Anyway"), style: .destructive) { _ in
            write(table: table, documentConfiguration: documentConfiguration,
                  configuration: configuration, baseName: baseName, allowAmbiguousOutput: true)
        })
        alert.addAction(UIAlertAction(title: String(localized: "Cancel"), style: .cancel))
        present(alert)
    }

    private static func write(table: CSVTable, documentConfiguration: CSVConfiguration,
                              configuration: CSVConfiguration, baseName: String,
                              allowAmbiguousOutput: Bool) {
        do {
            let url = try prepare(table: table, documentConfiguration: documentConfiguration,
                                  configuration: configuration, baseName: baseName,
                                  allowAmbiguousOutput: allowAmbiguousOutput)
            let picker = UIDocumentPickerViewController(forExporting: [url], asCopy: true)
            let cleanup = ExportFileCleanup(url.deletingLastPathComponent())
            picker.delegate = cleanup
            cleanups.append(cleanup)
            guard KeyWindow.present(picker) else {
                cleanup.finish()
                return
            }
        } catch let error as CSVError where error == .ambiguousOutput && !allowAmbiguousOutput {
            confirmAmbiguous(table: table, documentConfiguration: documentConfiguration,
                             configuration: configuration, baseName: baseName)
        } catch {
            presentError(FormatErrorText.alertMessage(for: error))
        }
    }

    private static func presentError(_ message: String) {
        let alert = UIAlertController(title: String(localized: "Export Not Completed"),
                                      message: message, preferredStyle: .alert)
        alert.addAction(UIAlertAction(title: String(localized: "OK"), style: .default))
        present(alert)
    }

    private static func dismissSheet(then completion: (() -> Void)? = nil) {
        let host = sheet
        sheet = nil
        if let host {
            host.dismiss(animated: true, completion: completion)
        } else {
            completion?()
        }
    }

    private static func present(_ controller: UIViewController) {
        KeyWindow.present(controller)
    }
}

private final class ExportFileCleanup: NSObject, UIDocumentPickerDelegate {
    let directory: URL

    init(_ directory: URL) {
        self.directory = directory
    }

    func documentPicker(_ controller: UIDocumentPickerViewController, didPickDocumentsAt urls: [URL]) {
        finish()
    }

    func documentPickerWasCancelled(_ controller: UIDocumentPickerViewController) {
        finish()
    }

    func finish() {
        try? FileManager.default.removeItem(at: directory)
        ExportCoordinator.drop(self)
    }
}

private extension ExportCoordinator {
    static func drop(_ cleanup: ExportFileCleanup) {
        cleanups.removeAll { $0 === cleanup }
    }
}

private struct ExportSheet: View {
    @ObservedObject var options: ExportOptions
    let onCancel: () -> Void
    let onExport: (CSVConfiguration) -> Void

    var body: some View {
        NavigationStack {
            ExportOptionsView(options: options) { _ in }
                .navigationTitle("Export")
                .toolbar {
                    ToolbarItem(placement: .cancellationAction) {
                        Button("Cancel", action: onCancel)
                    }
                    ToolbarItem(placement: .confirmationAction) {
                        Button("Export") { onExport(options.configuration) }
                    }
                }
        }
    }
}
