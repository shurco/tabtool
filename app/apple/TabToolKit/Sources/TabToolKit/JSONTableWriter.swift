/// Writes a table as a JSON array with one object per row.
///
/// Keys follow column order, so the writer builds the text itself instead of
/// using `JSONSerialization`, whose dictionaries do not keep key order.
/// Every value is a string: typing "007" or a long ID as a number would lose
/// data.
enum JSONTableWriter {
    static func string(table: CSVTable) -> String {
        let keys = ExportFieldNames.unique(ExportFieldNames.names(for: table)).map(quoted)
        guard table.rowCount > 0 else { return "[]\n" }

        var output = "[\n"
        for row in 0..<table.rowCount {
            let fields = keys.indices.map { column in
                "\(keys[column]): \(quoted(table.cell(row: row, column: column)))"
            }
            output += "  {" + fields.joined(separator: ", ") + "}"
            output += row == table.rowCount - 1 ? "\n" : ",\n"
        }
        return output + "]\n"
    }

    /// A JSON string literal for `text` (RFC 8259 escaping).
    static func quoted(_ text: String) -> String {
        var result = "\""
        for scalar in text.unicodeScalars {
            switch scalar {
            case "\"": result += "\\\""
            case "\\": result += "\\\\"
            case "\n": result += "\\n"
            case "\r": result += "\\r"
            case "\t": result += "\\t"
            case "\u{0}"..."\u{1F}":
                let hex = String(scalar.value, radix: 16, uppercase: true)
                result += "\\u" + String(repeating: "0", count: 4 - hex.count) + hex
            default: result.unicodeScalars.append(scalar)
            }
        }
        return result + "\""
    }
}
