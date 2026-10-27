import Foundation

/// Classifies and localizes numeric CSV cell text.
public enum CSVNumber {
    /// Returns `true` when `text` is a number using `decimalMark`.
    ///
    /// The pattern matches numeric cells: optional
    /// whitespace and sign, digits around the configured decimal mark, an
    /// optional exponent, and optional trailing whitespace.
    public static func isNumeric(_ text: String, decimalMark: String) -> Bool {
        guard let regex = numericRegex(decimalMark: decimalMark) else {
            return false
        }

        let range = NSRange(text.startIndex..<text.endIndex, in: text)
        return regex.firstMatch(in: text, range: range) != nil
    }

    // NSCache is internally thread-safe; it is just not marked Sendable.
    nonisolated(unsafe) private static let regexCache = NSCache<NSString, NSRegularExpression>()

    /// Compiled numeric pattern for `decimalMark`, cached per mark.
    private static func numericRegex(decimalMark: String) -> NSRegularExpression? {
        let key = decimalMark as NSString
        if let cached = regexCache.object(forKey: key) { return cached }

        let mark = NSRegularExpression.escapedPattern(for: decimalMark)
        let pattern = #"^\s*[+-]?(\d+\#(mark)?\d*|\d*\#(mark)?\d+)([eE][+-]?\d+)?\s*$"#
        guard let regex = try? NSRegularExpression(pattern: pattern) else { return nil }
        regexCache.setObject(regex, forKey: key)
        return regex
    }

    /// Returns `text` with the decimal mark replaced by the locale's decimal
    /// separator when `text` is numeric; otherwise returns `text` unchanged.
    public static func localized(_ text: String, decimalMark: String,
                                 locale: Locale = .current) -> String {
        guard isNumeric(text, decimalMark: decimalMark),
              let separator = locale.decimalSeparator else {
            return text
        }
        return text.replacingOccurrences(of: decimalMark, with: separator)
    }
}
