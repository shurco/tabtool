import TabToolKit

/// The value captured when a document is saved.
///
/// The BOM flag belongs to the snapshot so a write cannot pick up a BOM state
/// that changed after the snapshot was taken.
struct CSVDocumentSnapshot: Sendable, Equatable {
    var table: CSVTable
    var configuration: CSVConfiguration
    var includesUTF8BOM: Bool

    init(table: CSVTable, configuration: CSVConfiguration,
         includesUTF8BOM: Bool = false) {
        self.table = table
        self.configuration = configuration
        self.includesUTF8BOM = includesUTF8BOM
    }
}
