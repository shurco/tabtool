/// Writes a table as an XML document: `<rows>` holds one `<row>` per table
/// row, and each cell is an element named after its column.
enum XMLTableWriter {
    static func string(table: CSVTable) -> String {
        let names = ExportFieldNames.unique(
            ExportFieldNames.names(for: table).map(elementName))
        var output = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<rows>\n"
        for row in 0..<table.rowCount {
            output += "  <row>\n"
            for (column, name) in names.enumerated() {
                let value = escaped(table.cell(row: row, column: column))
                output += "    <\(name)>\(value)</\(name)>\n"
            }
            output += "  </row>\n"
        }
        return output + "</rows>\n"
    }

    /// A valid XML element name for a column name.
    ///
    /// Characters outside the XML 1.0 `NameChar` production become `_`; the
    /// colon is excluded too because it means a namespace prefix. A name that
    /// cannot start an element (digit, `-`, `.`) or starts with the reserved
    /// `xml` gets a leading `_`.
    static func elementName(_ name: String) -> String {
        var body = String.UnicodeScalarView()
        for scalar in name.unicodeScalars {
            body.append(isNameChar(scalar) ? scalar : "_")
        }
        let result = String(body)
        guard let first = body.first, isNameStart(first),
              !result.lowercased().hasPrefix("xml") else { return "_" + result }
        return result
    }

    /// XML 1.0 (Fifth Edition) `NameStartChar`, without the colon.
    private static func isNameStart(_ scalar: Unicode.Scalar) -> Bool {
        switch scalar.value {
        case 0x41...0x5A, 0x5F, 0x61...0x7A, 0xC0...0xD6, 0xD8...0xF6,
             0xF8...0x2FF, 0x370...0x37D, 0x37F...0x1FFF, 0x200C...0x200D,
             0x2070...0x218F, 0x2C00...0x2FEF, 0x3001...0xD7FF,
             0xF900...0xFDCF, 0xFDF0...0xFFFD, 0x10000...0xEFFFF:
            return true
        default:
            return false
        }
    }

    /// XML 1.0 (Fifth Edition) `NameChar`, without the colon.
    private static func isNameChar(_ scalar: Unicode.Scalar) -> Bool {
        switch scalar.value {
        case 0x2D, 0x2E, 0x30...0x39, 0xB7, 0x300...0x36F, 0x203F...0x2040:
            return true
        default:
            return isNameStart(scalar)
        }
    }

    /// Element text with markup characters escaped.
    ///
    /// Carriage returns become `&#13;` so parsers do not normalize them away.
    static func escaped(_ text: String) -> String {
        var result = ""
        for scalar in text.unicodeScalars {
            switch scalar {
            case "&": result += "&amp;"
            case "<": result += "&lt;"
            case ">": result += "&gt;"
            case "\r": result += "&#13;"
            case "\t", "\n": result.unicodeScalars.append(scalar)
            // ponytail: XML 1.0 has no way to encode these, even as references;
            // replaced with U+FFFD. Switch to XML 1.1 if users need them kept.
            case "\u{0}"..."\u{1F}", "\u{FFFE}", "\u{FFFF}": result += "\u{FFFD}"
            default: result.unicodeScalars.append(scalar)
            }
        }
        return result
    }
}
