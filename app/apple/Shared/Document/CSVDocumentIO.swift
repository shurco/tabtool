import Foundation
import TabToolKit

/// File access, parsing, and encoding shared by document loading, re-reading,
/// and saving.
enum CSVDocumentIO {
    /// A parsed table together with the BOM state of its source bytes.
    struct ParsedTable: Sendable {
        let table: CSVTable
        let includesUTF8BOM: Bool
    }

    /// Why an initial load did not produce a table.
    enum LoadFailure: Error, Equatable {
        case csv(CSVError)
        case cannotReadFile
    }

    /// The resolved configuration and the load result.
    struct LoadOutcome: Sendable {
        let configuration: CSVConfiguration
        let result: Result<ParsedTable, LoadFailure>
    }

    /// Detects the format when no configuration was supplied and parses the
    /// initial bytes or the current file.
    static func load(data: Data?, fileURL: URL?, configuration: CSVConfiguration?,
                     prefersChineseEncoding: Bool) -> LoadOutcome {
        let fallback = configuration ?? CSVConfiguration()
        let bytes: Data
        if let data {
            bytes = data
        } else {
            guard let fileURL else {
                return LoadOutcome(configuration: fallback,
                                   result: .failure(.cannotReadFile))
            }
            do {
                bytes = try readData(at: fileURL)
            } catch {
                return LoadOutcome(configuration: fallback,
                                   result: .failure(.cannotReadFile))
            }
        }

        let resolved = configuration
            ?? CSVFormatDetector(data: bytes,
                                 prefersChineseEncoding: prefersChineseEncoding).detect()
        do {
            let parsed = try parse(data: bytes, configuration: resolved)
            return LoadOutcome(configuration: resolved, result: .success(parsed))
        } catch let error as CSVError {
            return LoadOutcome(configuration: resolved, result: .failure(.csv(error)))
        } catch {
            return LoadOutcome(configuration: resolved,
                               result: .failure(.csv(.cannotDecodeData)))
        }
    }

    /// Reads the bytes at `url`, bracketing security-scoped access.
    ///
    /// Throws `DocumentError.cannotReadFile` for I/O failures so they stay
    /// distinct from `CSVError.cannotDecodeData`.
    static func readData(at url: URL) throws -> Data {
        let isAccessing = url.startAccessingSecurityScopedResource()
        defer {
            if isAccessing {
                url.stopAccessingSecurityScopedResource()
            }
        }
        do {
            return try Data(contentsOf: url)
        } catch {
            throw DocumentError.cannotReadFile
        }
    }

    /// Parses bytes into a logical table.
    ///
    /// The first row is extracted into headers when the configuration enables
    /// header mode. Parsing errors propagate as `CSVError`.
    static func parse(data: Data,
                      configuration: CSVConfiguration) throws -> ParsedTable {
        var parser = try CSVParser(data: data, configuration: configuration)
        let rows = try parser.readAll()
        let headers = configuration.firstRowAsHeader ? rows.first : nil
        let dataRows = headers == nil ? rows : Array(rows.dropFirst())
        return ParsedTable(table: CSVTable(rows: dataRows, headers: headers),
                           includesUTF8BOM: parser.hadUTF8BOM)
    }

    /// The explicit header row a save writes for `table`, or `nil`.
    static func header(for table: CSVTable,
                       configuration: CSVConfiguration) -> [String]? {
        configuration.firstRowAsHeader ? table.headers : nil
    }

    /// Maps a validation issue to the error a save throws.
    static func error(for issue: CSVWriteIssue) -> CSVError {
        switch issue {
        case .unencodableText:
            return .unencodableText
        case .ambiguousOutput:
            return .ambiguousOutput
        }
    }

    /// The typed error that blocks saving `snapshot`, or `nil` when writable.
    static func saveError(for snapshot: CSVDocumentSnapshot) -> CSVError? {
        let header = header(for: snapshot.table, configuration: snapshot.configuration)
        guard let issue = CSVWriteValidation.issue(table: snapshot.table, header: header,
                                                   configuration: snapshot.configuration) else {
            return nil
        }
        return error(for: issue)
    }

    /// Validates and serializes a snapshot without document or UI state.
    static func serializedData(for snapshot: CSVDocumentSnapshot) throws -> Data {
        if let error = saveError(for: snapshot) {
            throw error
        }
        return try encodedData(for: snapshot)
    }

    /// Serializes a snapshot with the BOM captured in that snapshot.
    static func encodedData(for snapshot: CSVDocumentSnapshot) throws -> Data {
        let header = header(for: snapshot.table, configuration: snapshot.configuration)
        return try CSVWriter(table: snapshot.table, header: header,
                             configuration: snapshot.configuration,
                             includeUTF8BOM: snapshot.includesUTF8BOM).data()
    }
}
