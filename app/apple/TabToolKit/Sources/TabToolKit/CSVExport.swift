import Foundation

/// Produces export bytes for a format and export configuration.
public enum CSVExport {
    /// Returns the first issue `data` would throw for the same converted
    /// table, header, and configuration, or `nil` when the export can be
    /// written. JSON and XML are UTF-8 and escape every value, so they never
    /// have an issue.
    ///
    /// The check runs after decimal-mark conversion because that conversion
    /// can add or remove the separator inside numeric cells.
    public static func preflightIssue(
        table: CSVTable,
        format: ExportFormat = .csv,
        documentConfiguration: CSVConfiguration,
        exportConfiguration: CSVConfiguration
    ) -> CSVWriteIssue? {
        guard format == .csv else { return nil }
        return CSVWriteValidation.issue(
            table: convertedTable(table, from: documentConfiguration,
                                  to: exportConfiguration),
            header: header(table: table, configuration: exportConfiguration),
            configuration: exportConfiguration
        )
    }

    /// Converts `table` to `format` and returns the encoded data.
    ///
    /// Numeric cells are copied and their decimal mark is changed from the
    /// document mark to the export mark; the source table is not modified.
    ///
    /// CSV: when the export configuration enables a header, the document
    /// headers are used, falling back to spreadsheet-style names (A, B, ...)
    /// for a table without headers. Quoting, encoding, and ambiguity checks
    /// follow `exportConfiguration`.
    ///
    /// JSON and XML: UTF-8, with field names from `ExportFieldNames`; only
    /// the decimal mark of `exportConfiguration` applies.
    public static func data(table: CSVTable,
                            format: ExportFormat = .csv,
                            documentConfiguration: CSVConfiguration,
                            exportConfiguration: CSVConfiguration,
                            allowAmbiguousOutput: Bool = false) throws -> Data {
        let converted = convertedTable(table, from: documentConfiguration,
                                       to: exportConfiguration)
        switch format {
        case .json:
            return Data(JSONTableWriter.string(table: converted).utf8)
        case .xml:
            return Data(XMLTableWriter.string(table: converted).utf8)
        case .csv:
            return try CSVWriter(
                table: converted,
                header: header(table: table, configuration: exportConfiguration),
                configuration: exportConfiguration,
                allowAmbiguousOutput: allowAmbiguousOutput
            ).data()
        }
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
