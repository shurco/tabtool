import Foundation

/// The CSV format used to read and write a document.
///
/// The defaults are UTF-8, comma
/// separator, quotes escaped by doubling, decimal point, no header row.
public struct CSVConfiguration: Sendable, Equatable {
    public var encoding: String.Encoding
    public var columnSeparator: String
    public var quoteCharacter: String
    public var escapeCharacter: String
    public var decimalMark: String
    public var firstRowAsHeader: Bool

    public init(encoding: String.Encoding = .utf8,
                columnSeparator: String = ",",
                quoteCharacter: String = "\"",
                escapeCharacter: String = "\"",
                decimalMark: String = ".",
                firstRowAsHeader: Bool = false) {
        self.encoding = encoding
        self.columnSeparator = columnSeparator
        self.quoteCharacter = quoteCharacter
        self.escapeCharacter = escapeCharacter
        self.decimalMark = decimalMark
        self.firstRowAsHeader = firstRowAsHeader
    }
}
