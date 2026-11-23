import TabToolKit

/// The value captured when a document is saved.
///
/// The BOM flag and line ending belong to the snapshot so a write cannot pick
/// up a state that changed after the snapshot was taken.
struct CSVDocumentSnapshot: Sendable, Equatable {
    var table: CSVTable
    var configuration: CSVConfiguration
    var includesUTF8BOM: Bool
    var lineEnding: CSVLineEnding

    init(table: CSVTable, configuration: CSVConfiguration,
         includesUTF8BOM: Bool = false,
         lineEnding: CSVLineEnding = .default) {
        self.table = table
        self.configuration = configuration
        self.includesUTF8BOM = includesUTF8BOM
        self.lineEnding = lineEnding
    }
}
