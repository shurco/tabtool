import Foundation

/// A problem that prevents a table from being written as CSV.
public enum CSVWriteIssue: Equatable, Sendable {
    /// A header or cell cannot be represented in the configured encoding.
    case unencodableText
    /// With quoting disabled, a header or cell would change the table
    /// structure: it contains the column separator or a line break.
    case ambiguousOutput
}

/// Checks whether a table can be saved with a configuration.
public enum CSVWriteValidation {
    /// Returns the first blocking issue, or `nil` when the table can be saved.
    ///
    /// The checks run in the same order as `CSVWriter.data()`: ambiguity first,
    /// then encodability of the exact text that saving would produce.
    public static func issue(table: CSVTable, header: [String]?,
                             configuration: CSVConfiguration) -> CSVWriteIssue? {
        if isAmbiguous(table: table, header: header, configuration: configuration) {
            return .ambiguousOutput
        }
        // Every Swift string is representable in UTF-8, which is the default
        // and keeps validation constant-time for ordinary documents.
        guard configuration.encoding != .utf8 else { return nil }

        let text = CSVWriter.serializedText(table: table, header: header,
                                            configuration: configuration)
        guard text.data(using: configuration.encoding) != nil else {
            return .unencodableText
        }
        return nil
    }

    /// Whether unquoted output would parse as a different table.
    static func isAmbiguous(table: CSVTable, header: [String]?,
                            configuration: CSVConfiguration) -> Bool {
        guard configuration.quoteCharacter.isEmpty else { return false }

        // A final empty single-column row contributes no characters after the
        // preceding row separator, so the output ends with LF and reparsing
        // drops that row. A lone empty cell is fine: empty output parses back
        // to one empty row.
        if table.columns.count == 1,
           let lastCell = table.columns.first?.cells.last,
           lastCell.isEmpty,
           header != nil || table.rowCount > 1 {
            return true
        }

        if let header, header.contains(where: {
            isAmbiguous($0, configuration: configuration)
        }) {
            return true
        }
        for column in table.columns {
            for cell in column.cells where isAmbiguous(cell, configuration: configuration) {
                return true
            }
        }
        return false
    }

    private static func isAmbiguous(_ field: String,
                                    configuration: CSVConfiguration) -> Bool {
        // A cheap byte scan clears almost every field: the first byte of the
        // separator and the lead bytes of LF, CR, NEL, LS and PS are the only
        // ways a field can contain one of them.
        let separatorByte = configuration.columnSeparator.utf8.first
        let mayBreak = field.utf8.contains {
            $0 == separatorByte || $0 == 0x0A || $0 == 0x0D || $0 == 0xC2 || $0 == 0xE2
        }
        guard mayBreak else { return false }

        let separator = configuration.columnSeparator
        if !separator.isEmpty, field.contains(separator) {
            return true
        }
        return field.unicodeScalars.contains(where: isRowBreak)
    }

    /// The row breaks recognized by `CSVParser`.
    private static func isRowBreak(_ scalar: Unicode.Scalar) -> Bool {
        switch scalar.value {
        case 0x0A, 0x0D, 0x85, 0x2028, 0x2029:
            return true
        default:
            return false
        }
    }
}
