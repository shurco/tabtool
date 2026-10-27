import Foundation

/// Reads CSV text row by row using a `CSVConfiguration`.
///
/// The parser scans an index over the decoded text and keeps its position
/// until `reset()` is called. It returns cell text without classifying or
/// converting values; callers interpret the strings.
public struct CSVParser {
    private let scalars: [Unicode.Scalar]
    private let columnSeparator: Unicode.Scalar?
    private let quoteCharacter: Unicode.Scalar?
    private let escapeCharacter: Unicode.Scalar?

    private var index = 0
    private var hasReturnedRow = false

    /// Whether the input data began with a UTF-8 byte order mark.
    public let hadUTF8BOM: Bool

    /// Whether all rows have been consumed.
    public var isAtEnd: Bool {
        hasReturnedRow && index >= scalars.count
    }

    /// Decodes `data` with the configuration's encoding and prepares a parser.
    ///
    /// A leading UTF-8 BOM is removed before decoding and reported through
    /// `hadUTF8BOM`. Throws `CSVError.cannotDecodeData` when the bytes are not
    /// valid in the configured encoding.
    public init(data: Data, configuration: CSVConfiguration) throws {
        let decoded = try Self.decode(data, encoding: configuration.encoding)
        self.init(scalars: decoded.scalars, configuration: configuration,
                  hadUTF8BOM: decoded.hadUTF8BOM)
    }

    /// Prepares a parser over an already decoded string.
    public init(string: String, configuration: CSVConfiguration) {
        self.init(scalars: Array(string.unicodeScalars), configuration: configuration,
                  hadUTF8BOM: false)
    }

    /// Prepares a parser over scalars from `decode(_:encoding:)`, so several
    /// parsers can share one decoded buffer.
    init(scalars: [Unicode.Scalar], configuration: CSVConfiguration, hadUTF8BOM: Bool) {
        self.scalars = scalars
        columnSeparator = configuration.columnSeparator.unicodeScalars.first
        quoteCharacter = configuration.quoteCharacter.unicodeScalars.first
        escapeCharacter = configuration.escapeCharacter.unicodeScalars.first
        self.hadUTF8BOM = hadUTF8BOM
    }

    /// Decodes `data` with `encoding`, removing a leading UTF-8 BOM.
    ///
    /// Throws `CSVError.cannotDecodeData` when the bytes are not valid in the
    /// encoding.
    static func decode(_ data: Data, encoding: String.Encoding) throws
        -> (scalars: [Unicode.Scalar], hadUTF8BOM: Bool) {
        var data = data
        let strippedBOM = encoding == .utf8 && data.starts(with: Data([0xEF, 0xBB, 0xBF]))
        if strippedBOM {
            data.removeFirst(3)
        }
        guard let text = String(data: data, encoding: encoding) else {
            throw CSVError.cannotDecodeData
        }
        return (Array(text.unicodeScalars), strippedBOM)
    }

    /// Restarts reading from the beginning of the input.
    public mutating func reset() {
        index = 0
        hasReturnedRow = false
    }

    /// Returns the next row, or `nil` when the input is exhausted.
    ///
    /// Throws `CSVError.malformedQuotedField` for an unterminated quoted field,
    /// a trailing escape, or content after a closing quote, and
    /// `CSVError.quoteInUnquotedField` when a quote appears in unquoted content
    /// while quotes are enabled.
    public mutating func nextRow() throws -> [String]? {
        guard !isAtEnd else { return nil }
        guard index < scalars.count else {
            hasReturnedRow = true
            return [""]
        }

        var row: [String] = []
        while true {
            row.append(try readField())
            if index == scalars.count {
                hasReturnedRow = true
                return row
            }
            if isRowBreak(scalars[index]) {
                consumeRowBreak()
                hasReturnedRow = true
                return row
            }
            // A field scan stops only at the separator or a row break.
            index += 1
            if index == scalars.count {
                row.append("")
                hasReturnedRow = true
                return row
            }
        }
    }

    /// Reads all remaining rows.
    public mutating func readAll() throws -> [[String]] {
        var rows: [[String]] = []
        while let row = try nextRow() {
            rows.append(row)
        }
        return rows
    }

    private mutating func readField() throws -> String {
        if let quoteCharacter, index < scalars.count, scalars[index] == quoteCharacter {
            return try readQuotedField(quote: quoteCharacter)
        }
        return try readUnquotedField()
    }

    private mutating func readQuotedField(quote: Unicode.Scalar) throws -> String {
        index += 1
        var field = String.UnicodeScalarView()

        while index < scalars.count {
            let scalar = scalars[index]

            // A dedicated escape character consumes the next scalar literally.
            if let escapeCharacter, escapeCharacter != quote, scalar == escapeCharacter {
                guard index + 1 < scalars.count else {
                    throw CSVError.malformedQuotedField
                }
                field.append(scalars[index + 1])
                index += 2
                continue
            }

            guard scalar == quote else {
                field.append(scalar)
                index += 1
                continue
            }

            // With the quote as its own escape, a doubled quote is one quote.
            if escapeCharacter == quote,
               index + 1 < scalars.count,
               scalars[index + 1] == quote {
                field.append(quote)
                index += 2
                continue
            }

            index += 1
            if index == scalars.count || isFieldTerminator(scalars[index]) {
                return String(field)
            }
            throw CSVError.malformedQuotedField
        }

        throw CSVError.malformedQuotedField
    }

    private mutating func readUnquotedField() throws -> String {
        var field = String.UnicodeScalarView()
        while index < scalars.count, !isFieldTerminator(scalars[index]) {
            if let quoteCharacter, scalars[index] == quoteCharacter {
                throw CSVError.quoteInUnquotedField
            }
            field.append(scalars[index])
            index += 1
        }
        return String(field)
    }

    private func isFieldTerminator(_ scalar: Unicode.Scalar) -> Bool {
        if let columnSeparator, scalar == columnSeparator {
            return true
        }
        return isRowBreak(scalar)
    }

    private func isRowBreak(_ scalar: Unicode.Scalar) -> Bool {
        switch scalar.value {
        case 0x0A, 0x0D, 0x85, 0x2028, 0x2029:
            return true
        default:
            return false
        }
    }

    private mutating func consumeRowBreak() {
        if scalars[index].value == 0x0D,
           index + 1 < scalars.count,
           scalars[index + 1].value == 0x0A {
            index += 2
        } else {
            index += 1
        }
    }
}
