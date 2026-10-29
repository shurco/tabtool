import SwiftUI
import TabToolKit

/// One open document. Owns the undo stack and autosave that `DocumentGroup`
/// used to provide.
struct DocumentEditorScreen: View {
    let file: DocumentFile

    @StateObject private var document: CSVDocument
    @State private var undoManager = UndoManager()
    @State private var saveTask: Task<Void, Never>?
    @State private var saveError: String?
    @Environment(\.scenePhase) private var scenePhase

    init(file: DocumentFile) {
        self.file = file
        _document = StateObject(wrappedValue: Self.makeDocument(for: file.url))
    }

    var body: some View {
        DocumentView(document: document, fileURL: file.url)
            .navigationTitle(file.name)
            .navigationBarTitleDisplayMode(.inline)
            .onAppear { document.setUndoManager(undoManager) }
            .onChange(of: document.mutationRevision) { _, _ in scheduleSave() }
            .onChange(of: scenePhase) { _, phase in
                if phase != .active { saveNow() }
            }
            .onDisappear { saveNow() }
            .alert("The file could not be saved", isPresented: Binding(
                get: { saveError != nil },
                set: { if !$0 { saveError = nil } }
            )) {
                Button("OK", role: .cancel) {}
            } message: {
                Text(verbatim: saveError ?? "")
            }
    }

    /// Autosave after a pause, so a burst of edits writes once.
    private func scheduleSave() {
        saveTask?.cancel()
        saveTask = Task { @MainActor in
            try? await Task.sleep(for: .milliseconds(800))
            guard !Task.isCancelled else { return }
            saveNow()
        }
    }

    /// Writes the current content. A blocked save is explained by the
    /// document's needs-attention banner, not by this alert.
    private func saveNow() {
        saveTask?.cancel()
        saveTask = nil
        guard !document.isLargeMode, document.hasContent,
              let url = document.fileURL, document.writeIssue == nil else { return }
        do {
            try document.encodedData().write(to: url, options: .atomic)
        } catch {
            saveError = error.localizedDescription
        }
    }

    private static func makeDocument(for url: URL) -> CSVDocument {
        let size = (try? FileManager.default.attributesOfItem(atPath: url.path)[.size] as? NSNumber)?
            .int64Value ?? 0
        if CSVDocument.usesLargeMode(forFileSize: size) {
            return CSVDocument(largeFileURL: url,
                               prefersChineseEncoding: AppLanguage.systemPrefersChinese())
        }
        let data = (try? Data(contentsOf: url)) ?? Data()
        return CSVDocument(data: data, fileURL: url,
                           prefersChineseEncoding: AppLanguage.systemPrefersChinese())
    }
}
