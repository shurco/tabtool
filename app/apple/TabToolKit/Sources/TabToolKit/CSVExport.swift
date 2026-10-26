import Foundation

/// Produces CSV bytes for an export configuration.
public enum CSVExport {
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
        var converted = table
        converted.convertDecimalMark(from: documentConfiguration.decimalMark,
                                     to: exportConfiguration.decimalMark)

        let header: [String]?
        if exportConfiguration.firstRowAsHeader {
            header = table.headers ?? table.columns.indices.map {
                ColumnNameGenerator.name(for: $0)
            }
        } else {
            header = nil
        }

        return try CSVWriter(table: converted, header: header,
                             configuration: exportConfiguration,
                             allowAmbiguousOutput: allowAmbiguousOutput).data()
    }
}
