/// Field names for structured exports (JSON keys, XML element names).
public enum ExportFieldNames {
    /// The document headers, or spreadsheet-style names (A, B, ...) for a
    /// table without headers. An empty header takes its column's name.
    /// Names are not deduplicated; see `unique(_:)`.
    public static func names(for table: CSVTable) -> [String] {
        table.columns.indices.map { index in
            let header = table.columns[index].header ?? ""
            return header.isEmpty ? ColumnNameGenerator.name(for: index) : header
        }
    }

    /// Makes repeated names distinct by appending `_2`, `_3`, ... while
    /// skipping any suffixed name that already appears in `names`.
    public static func unique(_ names: [String]) -> [String] {
        var taken = Set(names)
        var seen: Set<String> = []
        return names.map { name in
            guard !seen.insert(name).inserted else { return name }
            var counter = 2
            while taken.contains("\(name)_\(counter)") { counter += 1 }
            let renamed = "\(name)_\(counter)"
            taken.insert(renamed)
            return renamed
        }
    }
}
