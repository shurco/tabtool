import TabToolKit

/// The value captured when a document is saved.
struct CSVDocumentSnapshot: Sendable, Equatable {
    var table: CSVTable
    var configuration: CSVConfiguration
}
