import Foundation

/// Converts decimal marks in numeric text.
public enum DecimalMarkConverter {
    /// Returns `text` with `from` replaced by `to` when `text` is numeric
    /// using `from`; otherwise returns `text` unchanged.
    public static func convert(_ text: String, from: String, to: String) -> String {
        guard !from.isEmpty, from != to,
              CSVNumber.isNumeric(text, decimalMark: from) else {
            return text
        }
        return text.replacingOccurrences(of: from, with: to)
    }
}
