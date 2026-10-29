import UIKit
import UniformTypeIdentifiers

/// Save and Save As for a large file, plus the prompt when edits would be lost.
@MainActor
final class LargeSaveCoordinator: NSObject, UIDocumentPickerDelegate {
    static let shared = LargeSaveCoordinator()

    private var document: CSVDocument?
    private var folderScope: SecurityScope?
    private var isPrompting = false

    func save(_ document: CSVDocument) {
        Task { await write(document, to: nil) }
    }

    /// Asks for a folder, then writes a copy under the current file name.
    func saveAs(_ document: CSVDocument) {
        self.document = document
        let picker = UIDocumentPickerViewController(forOpeningContentTypes: [.folder], asCopy: false)
        picker.delegate = self
        KeyWindow.present(picker)
    }

    /// Asks before a large file's edits disappear. Cancel is offered only when
    /// the document stays open, such as when the app goes to the background.
    func presentUnsavedChanges(for document: CSVDocument, canCancel: Bool) {
        guard !isPrompting, document.isLargeMode, document.hasUnsavedLargeEdits,
              !document.isSaving else { return }
        isPrompting = true
        let name = document.fileURL?.lastPathComponent ?? "Untitled"
        let alert = UIAlertController(
            title: String(localized: "Do you want to save the changes made to “\(name)”?"),
            message: String(localized: "Your changes will be lost if you don’t save them."),
            preferredStyle: .alert)
        alert.addAction(UIAlertAction(title: String(localized: "Save"), style: .default) { [weak self] _ in
            self?.isPrompting = false
            self?.save(document)
        })
        alert.addAction(UIAlertAction(title: String(localized: "Don’t Save"), style: .destructive) { [weak self] _ in
            self?.isPrompting = false
            document.revertLargeEdits()
        })
        if canCancel {
            alert.addAction(UIAlertAction(title: String(localized: "Cancel"), style: .cancel) { [weak self] _ in
                self?.isPrompting = false
            })
        }
        if !KeyWindow.present(alert) {
            isPrompting = false
        }
    }

    func documentPicker(_ controller: UIDocumentPickerViewController, didPickDocumentsAt urls: [URL]) {
        guard let folder = urls.first, let document else { return }
        let name = document.fileURL?.lastPathComponent ?? "Untitled.csv"
        let destination = folder.appendingPathComponent(name)
        if destination.standardizedFileURL == document.fileURL?.standardizedFileURL {
            save(document)
            return
        }
        let scope = SecurityScope(folder)
        guard FileManager.default.fileExists(atPath: destination.path) else {
            Task { await write(document, to: destination, scope: scope) }
            return
        }
        confirmReplace(document: document, at: destination, scope: scope)
    }

    private func confirmReplace(document: CSVDocument, at destination: URL, scope: SecurityScope) {
        let alert = UIAlertController(
            title: String(localized: "Save"),
            message: destination.lastPathComponent,
            preferredStyle: .alert)
        alert.addAction(UIAlertAction(title: String(localized: "Cancel"), style: .cancel))
        alert.addAction(UIAlertAction(title: String(localized: "Save"), style: .destructive) { [weak self] _ in
            Task { await self?.write(document, to: destination, scope: scope) }
        })
        KeyWindow.present(alert)
    }

    private func write(_ document: CSVDocument, to destination: URL?, scope: SecurityScope? = nil) async {
        folderScope = scope
        defer { folderScope = nil }
        do {
            try await document.saveLargeFile(to: destination)
        } catch is CancellationError {
            return
        } catch {
            presentSaveError(error)
        }
    }

    private func presentSaveError(_ error: Error) {
        let alert = UIAlertController(
            title: String(localized: "The file could not be saved"),
            message: FormatErrorText.alertMessage(for: error),
            preferredStyle: .alert)
        alert.addAction(UIAlertAction(title: String(localized: "OK"), style: .default))
        KeyWindow.present(alert)
    }
}
