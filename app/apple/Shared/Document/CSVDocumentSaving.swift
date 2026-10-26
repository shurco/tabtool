import Foundation
import SwiftUI
import TabToolKit
import UniformTypeIdentifiers

/// Snapshot and file-wrapper serialization for saving.
extension CSVDocument {
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
}
