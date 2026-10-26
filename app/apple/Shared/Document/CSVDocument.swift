import SwiftUI
import TabToolKit
import UniformTypeIdentifiers

/// A CSV document backed by bytes read from disk.
@MainActor
final class CSVDocument: ObservableObject, @preconcurrency ReferenceFileDocument, @unchecked Sendable {
    typealias Snapshot = CSVDocumentSnapshot

    static var readableContentTypes: [UTType] {
        [.commaSeparatedText, .tabSeparatedText, .plainText]
    }

    @Published private(set) var table: CSVTable?
    @Published private(set) var configuration: CSVConfiguration
    @Published private(set) var phase: DocumentPhase
    @Published var selection = GridSelection()
    private(set) var fileURL: URL?

    /// Whether the loaded bytes began with a UTF-8 BOM. Captured in every
    /// snapshot so a save writes the BOM state the table was loaded with.
    private(set) var hadUTF8BOM = false

    /// Bytes supplied by the document initializer, used for the first load
    /// only. Later loads read the current file through `fileURL`.
    private var initialData: Data?
    private var shouldDetectFormat = false
    let prefersChineseEncoding: Bool
    private var undoManager: UndoManager?

    /// The load currently in flight, so overlapping `load()` calls share it.
    private var loadTask: Task<Void, Never>?

    init(data: Data, fileURL: URL? = nil, configuration: CSVConfiguration? = nil,
         prefersChineseEncoding: Bool = false, undoManager: UndoManager? = nil) {
        self.table = nil
        self.configuration = configuration ?? CSVConfiguration()
        self.phase = .loading
        self.fileURL = fileURL
        self.initialData = data
        self.shouldDetectFormat = configuration == nil
        self.prefersChineseEncoding = prefersChineseEncoding
        self.undoManager = undoManager
        undoManager?.groupsByEvent = false
    }

    init() {
        table = .newDocument()
        configuration = CSVConfiguration()
        phase = .ready
        fileURL = nil
        prefersChineseEncoding = false
    }

    convenience init(configuration: ReferenceFileDocument.ReadConfiguration) throws {
        guard let data = configuration.file.regularFileContents else {
            throw CSVError.cannotDecodeData
        }
        self.init(data: data, prefersChineseEncoding: Self.systemPrefersChineseEncoding)
    }

    private static var systemPrefersChineseEncoding: Bool {
        Locale.preferredLanguages.first?.lowercased().hasPrefix("zh") == true
    }

    /// The issue that blocks saving, or `nil` when the content is writable.
    var writeIssue: CSVWriteIssue? {
        if case .needsAttention(let issue) = phase {
            return issue
        }
        return nil
    }

    func setFileURL(_ url: URL?) {
        fileURL = url
    }

    /// Uses `undoManager` for document actions.
    ///
    /// Each action registers one explicit undo group, so a cell edit followed
    /// by an asynchronous re-read stays two separate undo steps even when they
    /// land in the same run-loop event. Event grouping is disabled for that
    /// reason; the document owns every registration on its manager.
    func setUndoManager(_ undoManager: UndoManager?) {
        self.undoManager = undoManager
        undoManager?.groupsByEvent = false
    }

    /// Detects the format and parses the bytes off the main actor.
    ///
    /// A supplied configuration skips detection. The initial bytes are used
    /// once; retrying after a read failure reads the current file through
    /// `fileURL`. A loaded document is a no-op. Overlapping calls await the
    /// same in-flight load instead of racing over `initialData`.
    func load() async {
        if let loadTask {
            await loadTask.value
            return
        }
        switch phase {
        case .ready, .needsAttention:
            return
        case .loading, .readFailed, .unreadableFile:
            break
        }

        let task = Task { await self.performLoad() }
        loadTask = task
        await task.value
        loadTask = nil
    }

    private func performLoad() async {
        let data = initialData
        initialData = nil
        let configuration = shouldDetectFormat ? nil : self.configuration
        shouldDetectFormat = false
        let fileURL = self.fileURL
        let prefersChinese = prefersChineseEncoding

        let outcome = await Task.detached(priority: .userInitiated) {
            CSVDocumentIO.load(data: data, fileURL: fileURL,
                               configuration: configuration,
                               prefersChineseEncoding: prefersChinese)
        }.value

        self.configuration = outcome.configuration
        switch outcome.result {
        case .success(let parsed):
            table = parsed.table
            hadUTF8BOM = parsed.includesUTF8BOM
            phase = .ready
            refreshPhase()
        case .failure(.csv(let error)):
            table = nil
            phase = .readFailed(error)
        case .failure(.cannotReadFile):
            table = nil
            phase = .unreadableFile
        }
    }

    func snapshot(contentType: UTType) throws -> CSVDocumentSnapshot {
        guard let table else { throw DocumentError.notReady }

        let header = CSVDocumentIO.header(for: table, configuration: configuration)
        if let issue = CSVWriteValidation.issue(table: table, header: header,
                                                configuration: configuration) {
            throw CSVDocumentIO.error(for: issue)
        }
        return CSVDocumentSnapshot(table: table, configuration: configuration,
                                   includesUTF8BOM: hadUTF8BOM)
    }

    /// Serializes the current content for saving and tests.
    func encodedData() throws -> Data {
        try CSVDocumentIO.encodedData(for: snapshot(contentType: .commaSeparatedText))
    }

    func fileWrapper(snapshot: CSVDocumentSnapshot,
                     configuration: ReferenceFileDocument.WriteConfiguration) throws -> FileWrapper {
        let header = CSVDocumentIO.header(for: snapshot.table,
                                          configuration: snapshot.configuration)
        if let issue = CSVWriteValidation.issue(table: snapshot.table, header: header,
                                                configuration: snapshot.configuration) {
            throw CSVDocumentIO.error(for: issue)
        }
        let data = try CSVDocumentIO.encodedData(for: snapshot)
        return FileWrapper(regularFileWithContents: data)
    }

    /// Replaces table, configuration, and BOM as one undoable user change.
    ///
    /// The editing, clipboard, and format extensions route every mutation
    /// through this method so phase refresh and undo recording stay in one
    /// place. A change with identical state records nothing.
    func applyChange(table newTable: CSVTable,
                     configuration newConfiguration: CSVConfiguration,
                     includesUTF8BOM newIncludesUTF8BOM: Bool,
                     actionName: String) {
        let previous = table.map { current in
            CSVDocumentSnapshot(table: current, configuration: configuration,
                                includesUTF8BOM: hadUTF8BOM)
        }
        if let previous,
           previous.table == newTable,
           previous.configuration == newConfiguration,
           previous.includesUTF8BOM == newIncludesUTF8BOM {
            return
        }

        table = newTable
        configuration = newConfiguration
        hadUTF8BOM = newIncludesUTF8BOM
        phase = .ready
        refreshPhase()

        if let previous {
            registerAction(actionName) { document in
                document.restore(snapshot: previous, actionName: actionName)
            }
        }
    }

    /// Replaces one cell and records only the old value for undo.
    ///
    /// Cell edits are the hot path, so they do not snapshot the whole table;
    /// the inverse edit captures the previous string instead.
    func applyCellChange(_ text: String, row: Int, column: Int, actionName: String) {
        guard let table,
              table.columns.indices.contains(column),
              table.columns[column].cells.indices.contains(row),
              table.cell(row: row, column: column) != text else {
            return
        }
        let previous = table.cell(row: row, column: column)
        var updated = table
        updated.setCell(text, row: row, column: column)
        self.table = updated
        phase = .ready
        refreshPhase()

        registerAction(actionName) { document in
            document.applyCellChange(previous, row: row, column: column,
                                     actionName: actionName)
        }
    }

    private func registerAction(
        _ actionName: String,
        _ handler: @escaping @MainActor (CSVDocument) -> Void
    ) {
        guard let undoManager else { return }
        undoManager.beginUndoGrouping()
        undoManager.registerUndo(withTarget: self, handler: handler)
        undoManager.setActionName(actionName)
        undoManager.endUndoGrouping()
    }

    /// Restores a snapshot and registers the inverse so redo works.
    private func restore(snapshot: CSVDocumentSnapshot, actionName: String) {
        guard let currentTable = table else { return }
        let previous = CSVDocumentSnapshot(table: currentTable,
                                           configuration: configuration,
                                           includesUTF8BOM: hadUTF8BOM)
        table = snapshot.table
        configuration = snapshot.configuration
        hadUTF8BOM = snapshot.includesUTF8BOM
        phase = .ready
        refreshPhase()
        registerAction(actionName) { document in
            document.restore(snapshot: previous, actionName: actionName)
        }
    }

    /// Recomputes `.ready`/`.needsAttention` after content or format changes.
    ///
    /// `.loading`, `.readFailed`, and `.unreadableFile` are sticky: there is
    /// no table to validate until the bytes load successfully. A successful
    /// re-read resets `.ready` before calling this.
    private func refreshPhase() {
        switch phase {
        case .loading, .readFailed, .unreadableFile:
            return
        case .ready, .needsAttention:
            break
        }
        guard let table else {
            phase = .loading
            return
        }
        let header = CSVDocumentIO.header(for: table, configuration: configuration)
        if let issue = CSVWriteValidation.issue(table: table, header: header,
                                                configuration: configuration) {
            phase = .needsAttention(issue)
        } else {
            phase = .ready
        }
    }

}
