import Foundation

/// Classifies and localizes numeric CSV cell text.
public enum CSVNumber {
    /// Returns `true` when `text` is a number using `decimalMark`.
    ///
    /// The pattern matches numeric cells: optional
    /// whitespace and sign, digits around the configured decimal mark, an
    /// optional exponent, and optional trailing whitespace.
    public static func isNumeric(_ text: String, decimalMark: String) -> Bool {
        scanNumeric(text, decimalMark: decimalMark)
            ?? matchesRegex(text, decimalMark: decimalMark)
    }

    /// Hand-written scan of the pattern for ASCII text and an ASCII decimal
    /// mark, about an order of magnitude faster than the regex. Returns `nil`
    /// when the input needs the regex (non-ASCII text, unusual mark).
    static func scanNumeric(_ text: String, decimalMark: String) -> Bool? {
        let mark = Array(decimalMark.utf8)
        guard mark.count == 1, let markByte = mark.first, markByte < 0x80,
              !isDigit(markByte), !isSpace(markByte),
              ![UInt8(ascii: "e"), UInt8(ascii: "E"), UInt8(ascii: "+"),
                UInt8(ascii: "-")].contains(markByte) else { return nil }
        return text.utf8.withContiguousStorageIfAvailable { bytes -> Bool? in
            if bytes.contains(where: { $0 >= 0x80 }) { return nil }
            var i = 0
            func skip(while predicate: (UInt8) -> Bool) -> Int {
                let start = i
                while i < bytes.count, predicate(bytes[i]) { i += 1 }
                return i - start
            }
            func consume(_ byte: UInt8) -> Bool {
                guard i < bytes.count, bytes[i] == byte else { return false }
                i += 1
                return true
            }
            _ = skip(while: isSpace)
            if !consume(UInt8(ascii: "+")) { _ = consume(UInt8(ascii: "-")) }
            let before = skip(while: isDigit)
            var after = 0
            if consume(markByte) { after = skip(while: isDigit) }
            guard before + after > 0 else { return false }
            if consume(UInt8(ascii: "e")) || consume(UInt8(ascii: "E")) {
                if !consume(UInt8(ascii: "+")) { _ = consume(UInt8(ascii: "-")) }
                guard skip(while: isDigit) > 0 else { return false }
            }
            _ = skip(while: isSpace)
            return i == bytes.count
        } ?? nil
    }

    private static func isDigit(_ byte: UInt8) -> Bool {
        byte >= UInt8(ascii: "0") && byte <= UInt8(ascii: "9")
    }

    /// The characters ICU's `\s` matches within ASCII.
    private static func isSpace(_ byte: UInt8) -> Bool {
        byte == 0x20 || (0x09...0x0D).contains(byte)
    }

    static func matchesRegex(_ text: String, decimalMark: String) -> Bool {
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
