import SwiftUI
import TabToolKit
import UniformTypeIdentifiers

/// A CSV document backed by bytes read from disk.
@MainActor
final class CSVDocument: ObservableObject, @preconcurrency ReferenceFileDocument, @unchecked Sendable {
    typealias Snapshot = CSVDocumentSnapshot

    /// The parse operation used by a load. Production always uses
    /// `CSVDocumentIO.load`; tests can suspend it to exercise cancellation.
    typealias Loader = @Sendable (Data?, URL?, CSVConfiguration?, Bool) -> CSVDocumentIO.LoadOutcome

    static var readableContentTypes: [UTType] {
        DroppedFiles.readableContentTypes
    }

    // A document may be created off the main actor, so the published state
    // starts at safe defaults and is filled in by the main-actor `load()`.
    @Published private(set) var table: CSVTable?
    @Published private(set) var configuration = CSVConfiguration()
    @Published private(set) var phase: DocumentPhase = .loading
    @Published var selection = GridSelection()

    /// The memory-mapped rows of a large file. `table` stays nil in large mode,
    /// which keeps every structural command disabled.
    @Published private(set) var largeSource: LargeCSVSource?
    @Published private(set) var largeStatus = LargeStatus.none

    /// Whether this document opens its file through the memory-mapped path.
    private(set) var isLargeMode = false

    /// Keeps the sandbox grant for the mapped file alive.
    private(set) var largeAccess: SecurityScope?

    /// The streaming save of a large file in flight, if any.
    var largeSave: Task<Void, Error>?

    /// A message the window shows once, set by the large file extension.
    @Published var notice: DocumentNotice?

    /// The active row sort, if any. Set by `applyChange(sort:)` and the
    /// sorting extension only; any other structural change clears it.
    @Published var activeSort: ActiveSort?
    private(set) var fileURL: URL?

    /// Whether the loaded bytes began with a UTF-8 BOM. Captured in every
    /// snapshot so a save writes the BOM state the table was loaded with.
    private(set) var hadUTF8BOM = false

    /// The save-time value. Autosave reads it off the main actor, so it is
    /// rebuilt after every change that can affect saving.
    let snapshotCache = CSVDocumentSnapshotCache()

    /// Increments after every content or file-binding change.
    ///
    /// A re-read captures this before its awaits and refuses to apply when it
    /// changed, so edits made while the re-read was parsing are not lost.
    private(set) var mutationRevision: UInt64 = 0

    /// What the initializer supplied, retained across cancelled attempts.
    ///
    /// Construction can run off the main actor, so an initializer stores plain
    /// values only and the main actor builds the visible document state. A
    /// data-only document (no file URL) has no other copy after its window
    /// closes, so its bytes are kept until a parse succeeds; a file-backed
    /// document releases them after a completed attempt and retries from disk.
    private enum InitialContent {
        case newDocument(makeConfiguration: @Sendable () -> CSVConfiguration)
        case data(Data, configuration: CSVConfiguration?)
        /// A file at or above the large file threshold; nothing is read until `load()`.
        case largeFile
    }
    private var initialContent: InitialContent?
    let prefersChineseEncoding: Bool
    private(set) var undoManager: UndoManager?
    private let loader: Loader

    /// The grid that owns the active field editor, if any.
    ///
    /// Weak because the coordinator belongs to the view hierarchy; the
    /// document outlives individual windows during re-reads and saving.
    weak var editingSession: DocumentEditingSession?

    /// The load currently in flight, so overlapping `load()` calls share it.
    private var loadTask: Task<LoadResult, Never>?

    /// Identifies the current attempt. Cancelling or starting a new attempt
    /// invalidates an older task's right to clear `loadTask`.
    private var loadGeneration: UInt64 = 0

    /// Cancels the initial load when its document window goes away.
    ///
    /// The supplied bytes stay in `initialContent` until a load completes
    /// without cancellation, so a data-only document can start a fresh parse
    /// when the window reopens. The cancelled task is detached from
    /// `loadTask` immediately; a later `load()` must not await it forever.
    func cancelLoading() {
        guard table == nil, let task = loadTask else { return }
        loadGeneration &+= 1
        task.cancel()
        loadTask = nil
    }

    /// Whether a file re-read is currently awaiting bytes or a parse.
    private var isRereading = false

    /// Creates a document around bytes supplied by the file reader.
    ///
    /// Runs off the main actor: it stores the bytes for the first main-actor
    /// `load()` instead of publishing document state.
    nonisolated init(data: Data, fileURL: URL? = nil, configuration: CSVConfiguration? = nil,
                     prefersChineseEncoding: Bool = false, undoManager: UndoManager? = nil,
                     loader: @escaping Loader = CSVDocumentIO.load) {
        self.fileURL = fileURL
        self.initialContent = .data(data, configuration: configuration)
        self.prefersChineseEncoding = prefersChineseEncoding
        self.undoManager = undoManager
        self.loader = loader
    }

    /// Creates a new document; `load()` materializes the table in the
    /// configuration `newDocumentConfiguration` returns.
    nonisolated init(
        newDocumentConfiguration: @escaping @Sendable () -> CSVConfiguration = { Preferences.newDocumentConfiguration() }
    ) {
        self.fileURL = nil
        self.initialContent = .newDocument(makeConfiguration: newDocumentConfiguration)
        self.prefersChineseEncoding = false
        self.undoManager = nil
        self.loader = CSVDocumentIO.load
    }

    /// Creates a document that maps its file instead of reading it.
    ///
    /// The URL may arrive later through `setFileURL`.
    nonisolated init(largeFileURL: URL?, prefersChineseEncoding: Bool = false) {
        self.fileURL = largeFileURL
        self.initialContent = .largeFile
        self.isLargeMode = true
        self.prefersChineseEncoding = prefersChineseEncoding
        // The document owns its undo stack: registering on the window's manager
        // would mark the document edited and make the system autosave (rewrite)
        // the whole file.
        self.undoManager = UndoManager()
        self.loader = CSVDocumentIO.load
    }

    nonisolated convenience init(configuration: ReferenceFileDocument.ReadConfiguration) throws {
        let size = (configuration.file.fileAttributes[FileAttributeKey.size.rawValue] as? NSNumber)?.int64Value ?? 0
        if Self.usesLargeMode(forFileSize: size) {
            self.init(largeFileURL: nil, prefersChineseEncoding: Self.systemPrefersChineseEncoding)
            return
        }
        guard let data = configuration.file.regularFileContents else {
            throw CSVError.cannotDecodeData
        }
        self.init(data: data, prefersChineseEncoding: Self.systemPrefersChineseEncoding)
    }

    private nonisolated static var systemPrefersChineseEncoding: Bool {
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
        guard url != fileURL else { return }
        fileURL = url
        mutationRevision &+= 1
    }

    /// Marks a re-read in flight and returns `false` when one is already
    /// running. Only one re-read may replace the document at a time.
    func beginReread() -> Bool {
        guard !isRereading else { return false }
        isRereading = true
        return true
    }

    /// Clears the in-flight re-read marker.
    func endReread() {
        isRereading = false
    }

    /// Uses `undoManager` for document actions.
    ///
    /// Each action registers one explicit undo group, so a cell edit followed
    /// by an asynchronous re-read stays two separate undo steps even when they
    /// land in the same run-loop event. Event grouping is disabled for that
    /// reason; the document owns every registration on its manager.
    func setUndoManager(_ undoManager: UndoManager?) {
        guard !isLargeMode else { return }
        self.undoManager = undoManager
        undoManager?.groupsByEvent = false
    }

    /// Detects the format and parses the bytes off the main actor.
    ///
    /// A new document becomes a 3×1 ready table. A supplied configuration
    /// skips detection. Initial bytes stay available until an attempt
    /// finishes without cancellation: a data-only document keeps them until
    /// a parse succeeds, while a file-backed document releases them after a
    /// failure and retries from `fileURL`. A loaded document is a no-op.
    /// Overlapping calls await the same in-flight load instead of racing over
    /// the initial content. A cancelled task is never awaited a second time:
    /// the next call starts a fresh attempt.
    func load() async {
        if let task = loadTask {
            await task.value
            return
        }
        switch phase {
        case .ready, .needsAttention:
            return
        case .loading, .readFailed, .unreadableFile:
            break
        }

        // The undo manager may arrive with the initializer or from the view;
        // either way it is configured once the document is on the main actor.
        undoManager?.groupsByEvent = false

        switch initialContent {
        case .newDocument(let makeConfiguration):
            initialContent = nil
            materializeNewDocument(configuration: makeConfiguration())
        case .data(let data, let configuration):
            // A data-only document has no other copy if its window closes.
            // File-backed documents can safely retry from the file on disk.
            let policy: InitialContentPolicy = fileURL == nil
                ? .retainUntilSuccess
                : .releaseAfterCompletedAttempt
            await performSharedLoad(data: data, configuration: configuration,
                                    initialContentPolicy: policy)
        case .largeFile:
            let task = Task { await self.openLargeFile(); return LoadResult.success }
            loadTask = task
            _ = await task.value
            loadTask = nil
        case nil:
            await performSharedLoad(data: nil, configuration: self.configuration,
                                    initialContentPolicy: nil)
        }
    }

    /// Publishes the outcome of opening a large file.
    ///
    /// `table` stays nil; the phase is not refreshed because there is no table
    /// to validate.
    func publishLarge(source: LargeCSVSource?, configuration: CSVConfiguration,
                      phase newPhase: DocumentPhase, access: SecurityScope?) {
        largeSource = source
        largeAccess = access
        self.configuration = configuration
        phase = newPhase
        largeStatus = source.map { $0.isIndexed ? .ready : .indexing($0.indexingFraction) } ?? .none
        mutationRevision &+= 1
    }

    /// Marks a content change made by the large file extension, so a re-read
    /// that started earlier does not overwrite it.
    func noteContentChanged() {
        mutationRevision &+= 1
    }

    func setLargeStatus(_ status: LargeStatus) {
        if largeStatus != status { largeStatus = status }
    }

    /// How a load attempt treats the initializer's bytes.
    private enum InitialContentPolicy {
        /// Keep bytes until a successful parse (data-only documents).
        case retainUntilSuccess
        /// Release after a successful or failed parse; the file URL is the
        /// retry source. A cancelled attempt releases nothing.
        case releaseAfterCompletedAttempt
    }

    /// The outcome of one parse attempt, used only to decide whether the
    /// initial bytes can be released.
    private enum LoadResult {
        case success
        case failure
        case cancelled
    }

    /// Builds the table for a new document; `load()` is idempotent.
    private func materializeNewDocument(configuration newConfiguration: CSVConfiguration) {
        var newTable = CSVTable.newDocument()
        if newConfiguration.firstRowAsHeader {
            newTable.setHeaders(Array(repeating: "", count: newTable.columns.count))
        }
        table = newTable
        configuration = newConfiguration
        phase = .ready
        refreshPhase()
    }

    private func performSharedLoad(data: Data?, configuration: CSVConfiguration?,
                                   initialContentPolicy: InitialContentPolicy?) async {
        loadGeneration &+= 1
        let generation = loadGeneration
        let task = Task { await self.performLoad(data: data, configuration: configuration) }
        loadTask = task
        let result = await task.value

        // A cancellation or a newer attempt already detached this task. It
        // must not clear the newer attempt's task or its retained bytes.
        guard generation == loadGeneration else { return }
        loadTask = nil

        guard let initialContentPolicy else { return }
        switch (initialContentPolicy, result) {
        case (.retainUntilSuccess, .success),
             (.releaseAfterCompletedAttempt, .success),
             (.releaseAfterCompletedAttempt, .failure):
            initialContent = nil
        case (.retainUntilSuccess, .failure), (_, .cancelled):
            break
        }
    }

    private func performLoad(data: Data?, configuration: CSVConfiguration?) async -> LoadResult {
        let fileURL = self.fileURL
        let prefersChinese = prefersChineseEncoding
        let loader = self.loader

        let outcome = await Task.detached(priority: .userInitiated) {
            loader(data, fileURL, configuration, prefersChinese)
        }.value

        // The window may have closed while the bytes were parsed. Do not
        // publish a result into a document that is no longer visible.
        guard !Task.isCancelled else { return .cancelled }
        self.configuration = outcome.configuration
        let result: LoadResult
        switch outcome.result {
        case .success(let parsed):
            table = parsed.table
            hadUTF8BOM = parsed.includesUTF8BOM
            phase = .ready
            refreshPhase()
            result = .success
        case .failure(.csv(let error)):
            table = nil
            phase = .readFailed(error)
            refreshSnapshotCache()
            result = .failure
        case .failure(.cannotReadFile):
            table = nil
            phase = .unreadableFile
            refreshSnapshotCache()
            result = .failure
        }
        mutationRevision &+= 1
        return result
    }

    /// Commits any in-progress cell edit before the caller changes the table.
    ///
    /// Structural actions, format changes, save key equivalents, and window
    /// close call this through their entry points. The field editor's row and
    /// column indexes describe the table before the mutation, so committing
    /// first prevents text from landing under a reused or shifted cell.
    func endEditing() {
        editingSession?.endEditing()
    }

    /// Replaces table, configuration, and BOM as one undoable user change.
    ///
    /// The editing, clipboard, and format extensions route every mutation
    /// through this method so phase refresh and undo recording stay in one
    /// place. A change with identical state records nothing, except for a
    /// column identity or order change: equal cell values cannot distinguish
    /// moving one of several identical columns.
    func applyChange(table newTable: CSVTable,
                     configuration newConfiguration: CSVConfiguration,
                     includesUTF8BOM newIncludesUTF8BOM: Bool,
                     actionName: String,
                     sort newSort: ActiveSort? = nil) {
        endEditing()
        let previousSort = activeSort
        let previous = table.map { current in
            CSVDocumentSnapshot(table: current, configuration: configuration,
                                includesUTF8BOM: hadUTF8BOM)
        }
        if let previous,
           previous.table == newTable,
           previous.table.hasSameColumnOrder(as: newTable),
           previous.configuration == newConfiguration,
           previous.includesUTF8BOM == newIncludesUTF8BOM {
            return
        }

        table = newTable
        configuration = newConfiguration
        hadUTF8BOM = newIncludesUTF8BOM
        activeSort = newSort
        validateSelection()
        phase = .ready
        refreshPhase()
        mutationRevision &+= 1

        if let previous {
            registerAction(actionName) { document in
                document.restore(snapshot: previous, sort: previousSort,
                                 actionName: actionName)
            }
        }
    }

    /// Replaces one cell and records only the old value for undo.
    ///
    /// Cell edits are the hot path, so they do not snapshot the whole table;
    /// the inverse edit captures the previous string instead.
    func applyCellChange(_ text: String, row: Int, column: Int, actionName: String) {
        if let source = largeSource {
            applyLargeCellChange(text, in: source, row: row, column: column, actionName: actionName)
            return
        }
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
        mutationRevision &+= 1

        registerAction(actionName) { document in
            document.applyCellChange(previous, row: row, column: column,
                                     actionName: actionName)
        }
    }

    func registerAction(
        _ actionName: String,
        _ handler: @escaping @MainActor (CSVDocument) -> Void
    ) {
        guard let undoManager else { return }
        undoManager.beginUndoGrouping()
        undoManager.registerUndo(withTarget: self, handler: handler)
        undoManager.setActionName(Bundle.main.localized(actionName))
        undoManager.endUndoGrouping()
    }

    /// Restores a snapshot and registers the inverse so redo works.
    private func restore(snapshot: CSVDocumentSnapshot, sort: ActiveSort?,
                         actionName: String) {
        endEditing()
        guard let currentTable = table else { return }
        let currentSort = activeSort
        let previous = CSVDocumentSnapshot(table: currentTable,
                                           configuration: configuration,
                                           includesUTF8BOM: hadUTF8BOM)
        table = snapshot.table
        configuration = snapshot.configuration
        hadUTF8BOM = snapshot.includesUTF8BOM
        activeSort = sort
        validateSelection()
        phase = .ready
        refreshPhase()
        mutationRevision &+= 1
        registerAction(actionName) { document in
            document.restore(snapshot: previous, sort: currentSort,
                             actionName: actionName)
        }
    }

    /// Drops row or column indexes that no longer exist.
    ///
    /// Structural edits and undo keep the selection where practical; indexes
    /// trimmed by a delete or an undo cannot stay selected.
    private func validateSelection() {
        guard let table else {
            selection = GridSelection()
            return
        }
        let rows = selection.rows.intersection(IndexSet(integersIn: 0..<table.rowCount))
        let columns = selection.columns.intersection(IndexSet(integersIn: 0..<table.columns.count))
        guard rows != selection.rows || columns != selection.columns else { return }
        selection = GridSelection(rows: rows, columns: columns)
    }

    /// Recomputes `.ready`/`.needsAttention` after content or format changes.
    ///
    /// `.loading`, `.readFailed`, and `.unreadableFile` are sticky: there is
    /// no table to validate until the bytes load successfully. A successful
    /// re-read resets `.ready` before calling this.
    private func refreshPhase() {
        switch phase {
        case .loading, .readFailed, .unreadableFile:
            // Sticky: there is no table to validate until bytes load.
            break
        case .ready, .needsAttention:
            if let table {
                let header = CSVDocumentIO.header(for: table, configuration: configuration)
                if let issue = CSVWriteValidation.issue(table: table, header: header,
                                                        configuration: configuration) {
                    phase = .needsAttention(issue)
                } else {
                    phase = .ready
                }
            } else {
                phase = .loading
            }
        }
        refreshSnapshotCache()
    }

}
