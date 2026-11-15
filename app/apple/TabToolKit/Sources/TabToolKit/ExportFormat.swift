/// The file format an export writes.
public enum ExportFormat: String, CaseIterable, Identifiable, Sendable {
    /// Delimited text that follows the export `CSVConfiguration`.
    case csv
    /// A UTF-8 JSON array with one object per row.
    case json
    /// A UTF-8 XML document with one element per row.
    case xml

    public var id: Self { self }
}
