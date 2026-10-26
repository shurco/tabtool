import Foundation

/// Produces CSV bytes for an export configuration.
public enum CSVExport {
    /// Returns the first issue `data` would throw for the same converted
    /// table, header, and configuration, or `nil` when the export can be
    /// written.
    ///
    /// The check runs after decimal-mark conversion because that conversion
    /// can add or remove the separator inside numeric cells.
    public static func preflightIssue(
        table: CSVTable,
        documentConfiguration: CSVConfiguration,
        exportConfiguration: CSVConfiguration
    ) -> CSVWriteIssue? {
        CSVWriteValidation.issue(
            table: convertedTable(table, from: documentConfiguration,
                                  to: exportConfiguration),
            header: header(table: table, configuration: exportConfiguration),
            configuration: exportConfiguration
        )
    }

    /// Converts `table` to the export format and returns the encoded data.
    ///
    /// Numeric cells are copied and their decimal mark is changed from the
    /// document mark to the export mark; the source table is not modified.
    /// When the export configuration enables a header, the document headers
    /// are used, falling back to spreadsheet-style names (A, B, ...) for a
    /// table without headers. Quoting, encoding, and ambiguity checks follow
    /// `exportConfiguration`.
    public static func data(table: CSVTable,
                            documentConfiguration: CSVConfiguration,
                            exportConfiguration: CSVConfiguration,
                            allowAmbiguousOutput: Bool = false) throws -> Data {
        try CSVWriter(
            table: convertedTable(table, from: documentConfiguration,
                                  to: exportConfiguration),
            header: header(table: table, configuration: exportConfiguration),
            configuration: exportConfiguration,
            allowAmbiguousOutput: allowAmbiguousOutput
        ).data()
    }

    /// The table the writer receives: the source with decimal marks converted.
    private static func convertedTable(
        _ table: CSVTable,
        from documentConfiguration: CSVConfiguration,
        to exportConfiguration: CSVConfiguration
    ) -> CSVTable {
        var converted = table
        converted.convertDecimalMark(from: documentConfiguration.decimalMark,
                                     to: exportConfiguration.decimalMark)
        return converted
    }

    /// The explicit header row an export writes, or `nil`.
    private static func header(table: CSVTable,
                               configuration: CSVConfiguration) -> [String]? {
        guard configuration.firstRowAsHeader else { return nil }
        return table.headers ?? table.columns.indices.map {
            ColumnNameGenerator.name(for: $0)
        }
    }
}
