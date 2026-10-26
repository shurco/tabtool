import Foundation

/// Converts between table cells and tab-separated clipboard text.
public enum CSVClipboard {
    /// Parses tab-separated text without quote handling.
    ///
    /// CRLF, CR, and LF each break rows. Empty fields and blank rows are
    /// preserved. A single trailing row break does not create an extra row;
    /// any further trailing breaks remain as empty rows.
    public static func parseTSV(_ string: String) -> [[String]] {
        let normalized = string
            .replacingOccurrences(of: "\r\n", with: "\n")
            .replacingOccurrences(of: "\r", with: "\n")
        var rows = normalized.split(separator: "\n", omittingEmptySubsequences: false)
        if normalized.hasSuffix("\n") {
            rows.removeLast()
        }
        return rows.map { row in
            row.split(separator: "\t", omittingEmptySubsequences: false).map(String.init)
        }
    }

    /// Produces tab-separated text with rows joined by line feeds.
    ///
    /// Tabs and line breaks inside cells become spaces so they cannot break
    /// the clipboard grid. Numeric cells are localized to the decimal
    /// separator of `locale`. Rows are joined without a separator after the
    /// last row.
    public static func makeTSV(rows: [[String]], numericDecimalMark: String,
                               locale: Locale = .current) -> String {
        rows.map { row in
            row.map { cell in
                let text = cell
                    .replacingOccurrences(of: "\r\n", with: " ")
                    .replacingOccurrences(of: "\r", with: " ")
                    .replacingOccurrences(of: "\n", with: " ")
                    .replacingOccurrences(of: "\t", with: " ")
                return CSVNumber.localized(text, decimalMark: numericDecimalMark,
                                           locale: locale)
            }
            .joined(separator: "\t")
        }
        .joined(separator: "\n")
    }
}
