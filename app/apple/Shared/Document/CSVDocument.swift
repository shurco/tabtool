import SwiftUI
import TabToolKit
import UniformTypeIdentifiers

/// Why the document cannot currently be edited or saved.
enum DocumentPhase: Equatable {
    /// Bytes are read, format detection and parsing are still running.
    case loading
    /// The table can be displayed, edited, and saved.
    case ready
    /// Reading the file failed; the format must be fixed before retrying.
    case readFailed(CSVError)
    /// The table is loaded but cannot be written safely with its configuration.
    case needsAttention(CSVWriteIssue)
}

/// Document operations that fail for reasons unrelated to CSV parsing.
enum DocumentError: Error, Equatable {
    /// The table is not loaded yet or the file could not be read.
    case notReady
    /// The operation would discard unsaved edits.
    case requiresConfirmation
    /// The file bytes are unavailable to the document.
    case cannotReadFile
}

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

    /// Bytes supplied by the document initializer, used for the first load
    /// only. Later loads read the current file through `fileURL`.
    private var initialData: Data?
    private var shouldDetectFormat = false
    private let prefersChineseEncoding: Bool
    private var hadUTF8BOM = false
    private var undoManager: UndoManager?

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

    func setUndoManager(_ undoManager: UndoManager?) {
        self.undoManager = undoManager
    }

    /// Detects the format and parses the bytes off the main actor.
    ///
    /// A supplied configuration skips detection. The initial bytes are used
    /// once; retrying after a read failure reads the current file through
    /// `fileURL`. A loaded document is a no-op.
    func load() async {
        switch phase {
        case .loading, .readFailed:
            break
        case .ready, .needsAttention:
            return
        }

        let data = initialData
        initialData = nil
        let configuration = shouldDetectFormat ? nil : self.configuration
        shouldDetectFormat = false
        let fileURL = fileURL
        let prefersChinese = prefersChineseEncoding

        let outcome = await Task.detached(priority: .userInitiated) {
            guard let data = data ?? Self.readFile(at: fileURL) else {
                return LoadOutcome(configuration: configuration ?? CSVConfiguration(),
                                   result: .failure(.cannotDecodeData))
            }
            return Self.parse(data: data, configuration: configuration,
                              prefersChineseEncoding: prefersChinese)
        }.value

        self.configuration = outcome.configuration
        switch outcome.result {
        case .success(let parsed):
            table = parsed.table
            hadUTF8BOM = parsed.hadUTF8BOM
            phase = .ready
            refreshPhase()
        case .failure(let error):
            table = nil
            phase = .readFailed(error)
        }
    }

    func snapshot(contentType: UTType) throws -> CSVDocumentSnapshot {
        guard let table else { throw DocumentError.notReady }

        let header = Self.header(for: table, configuration: configuration)
        if let issue = CSVWriteValidation.issue(table: table, header: header,
                                                configuration: configuration) {
            throw Self.error(for: issue)
        }
        return CSVDocumentSnapshot(table: table, configuration: configuration)
    }

    /// Serializes the current content for saving and tests.
    func encodedData() throws -> Data {
        let snapshot = try snapshot(contentType: .commaSeparatedText)
        return try Self.encodedData(for: snapshot, includeUTF8BOM: hadUTF8BOM)
    }

    func fileWrapper(snapshot: CSVDocumentSnapshot,
                     configuration: ReferenceFileDocument.WriteConfiguration) throws -> FileWrapper {
        let header = Self.header(for: snapshot.table, configuration: snapshot.configuration)
        if let issue = CSVWriteValidation.issue(table: snapshot.table, header: header,
                                                configuration: snapshot.configuration) {
            throw Self.error(for: issue)
        }
        let data = try Self.encodedData(for: snapshot, includeUTF8BOM: hadUTF8BOM)
        return FileWrapper(regularFileWithContents: data)
    }

    /// Recomputes `.ready`/`.needsAttention` after content or format changes.
    ///
    /// `.loading` and `.readFailed` are sticky: there is no table to validate
    /// until the bytes load successfully.
    private func refreshPhase() {
        switch phase {
        case .loading, .readFailed:
            return
        case .ready, .needsAttention:
            break
        }
        guard let table else {
            phase = .loading
            return
        }
        let header = Self.header(for: table, configuration: configuration)
        if let issue = CSVWriteValidation.issue(table: table, header: header,
                                                configuration: configuration) {
            phase = .needsAttention(issue)
        } else {
            phase = .ready
        }
    }

    private static func header(for table: CSVTable,
                               configuration: CSVConfiguration) -> [String]? {
        configuration.firstRowAsHeader ? table.headers : nil
    }

    private static func error(for issue: CSVWriteIssue) -> CSVError {
        switch issue {
        case .unencodableText:
            return .unencodableText
        case .ambiguousOutput:
            return .ambiguousOutput
        }
    }

    private static func encodedData(for snapshot: CSVDocumentSnapshot,
                                    includeUTF8BOM: Bool) throws -> Data {
        let header = header(for: snapshot.table, configuration: snapshot.configuration)
        return try CSVWriter(table: snapshot.table, header: header,
                             configuration: snapshot.configuration,
                             includeUTF8BOM: includeUTF8BOM).data()
    }

    private nonisolated static func readFile(at url: URL?) -> Data? {
        guard let url else { return nil }
        let isAccessing = url.startAccessingSecurityScopedResource()
        defer {
            if isAccessing {
                url.stopAccessingSecurityScopedResource()
            }
        }
        return try? Data(contentsOf: url)
    }

    private nonisolated static func parse(
        data: Data, configuration: CSVConfiguration?, prefersChineseEncoding: Bool
    ) -> LoadOutcome {
        let resolved = configuration
            ?? CSVFormatDetector(data: data, prefersChineseEncoding: prefersChineseEncoding).detect()

        do {
            var parser = try CSVParser(data: data, configuration: resolved)
            let rows = try parser.readAll()
            let headers = resolved.firstRowAsHeader ? rows.first : nil
            let dataRows = headers == nil ? rows : Array(rows.dropFirst())
            let table = CSVTable(rows: dataRows, headers: headers)
            return LoadOutcome(configuration: resolved,
                               result: .success(ParsedTable(table: table,
                                                            hadUTF8BOM: parser.hadUTF8BOM)))
        } catch let error as CSVError {
            return LoadOutcome(configuration: resolved, result: .failure(error))
        } catch {
            return LoadOutcome(configuration: resolved, result: .failure(.cannotDecodeData))
        }
    }

    private struct ParsedTable: Sendable {
        let table: CSVTable
        let hadUTF8BOM: Bool
    }

    private struct LoadOutcome: Sendable {
        let configuration: CSVConfiguration
        let result: Result<ParsedTable, CSVError>
    }
}
