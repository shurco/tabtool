import Foundation

/// Serializes a `CSVTable` with an explicit header row and configuration.
///
/// The writer only formats text. Numeric cells and decimal marks reach it as
/// plain strings; callers that need conversion use `CSVExport`. Fields follow
/// the table's visual column order. Quoting is enabled whenever
/// `configuration.quoteCharacter` is not empty, and every field is quoted when
/// enabled. Rows are separated by a line feed and the output has no trailing
/// newline. The header row is written as provided, so callers normally pass
/// `table.headers` or `nil` to match the document's header toggle.
public struct CSVWriter: Sendable {
    private static let utf8BOM = Data([0xEF, 0xBB, 0xBF])

    private let table: CSVTable
    private let header: [String]?
    private let configuration: CSVConfiguration
    private let includeUTF8BOM: Bool
    private let allowAmbiguousOutput: Bool

    public init(table: CSVTable, header: [String]?,
                configuration: CSVConfiguration,
                includeUTF8BOM: Bool = false,
                allowAmbiguousOutput: Bool = false) {
        self.table = table
        self.header = header
        self.configuration = configuration
        self.includeUTF8BOM = includeUTF8BOM
        self.allowAmbiguousOutput = allowAmbiguousOutput
    }

    /// Returns the serialized CSV text without a trailing newline.
    ///
    /// Throws `CSVError.ambiguousOutput` when quoting is disabled and a field
    /// contains the column separator or a line break, unless
    /// `allowAmbiguousOutput` was set.
    public func string() throws -> String {
        if !allowAmbiguousOutput,
           CSVWriteValidation.isAmbiguous(table: table, header: header,
                                          configuration: configuration) {
            throw CSVError.ambiguousOutput
        }
        return Self.serializedText(table: table, header: header,
                                   configuration: configuration)
    }

    /// Encodes `string()` with the configuration's encoding.
    ///
    /// A UTF-8 BOM is prepended when requested and the encoding is UTF-8.
    /// Throws `CSVError.unencodableText` when the output cannot be represented
    /// in the configured encoding.
    public func data() throws -> Data {
        let text = try string()
        guard let encoded = text.data(using: configuration.encoding) else {
            throw CSVError.unencodableText
        }
        guard includeUTF8BOM, configuration.encoding == .utf8 else {
            return encoded
        }
        var data = Self.utf8BOM
        data.append(encoded)
        return data
    }

    /// Serializes without ambiguity enforcement so validation can check the
    /// exact text that saving would encode.
    static func serializedText(table: CSVTable, header: [String]?,
                               configuration: CSVConfiguration) -> String {
        var text = ""
        var isFirstRow = true

        if let header {
            appendRow(header, to: &text, isFirstRow: &isFirstRow,
                      configuration: configuration)
        }
        for row in 0..<table.rowCount {
            appendRow(table.columns.map { $0.cells[row] },
                      to: &text, isFirstRow: &isFirstRow,
                      configuration: configuration)
        }
        return text
    }

    private static func appendRow(_ fields: [String], to text: inout String,
                                  isFirstRow: inout Bool,
                                  configuration: CSVConfiguration) {
        if !isFirstRow {
            text.append("\n")
        }
        isFirstRow = false

        for index in fields.indices {
            if index > 0 {
                text.append(configuration.columnSeparator)
            }
            appendField(fields[index], to: &text, configuration: configuration)
        }
    }

    private static func appendField(_ field: String, to text: inout String,
                                    configuration: CSVConfiguration) {
        let quote = configuration.quoteCharacter
        guard !quote.isEmpty else {
            text.append(field)
            return
        }

        // Mirror the parser: a dedicated escape character is doubled so it
        // stays literal, and a quote is prefixed with that escape. When the
        // quote is its own escape, the quote is doubled instead.
        let escape = configuration.escapeCharacter
        var escaped = field
        if escape == quote {
            escaped = escaped.replacingOccurrences(of: quote, with: quote + quote)
        } else {
            if !escape.isEmpty {
                escaped = escaped.replacingOccurrences(of: escape, with: escape + escape)
            }
            escaped = escaped.replacingOccurrences(of: quote, with: escape + quote)
        }

        text.append(quote)
        text.append(escaped)
        text.append(quote)
    }
}
