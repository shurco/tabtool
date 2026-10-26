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
    /// `hadUTF8BOM`.
    public init(data: Data, configuration: CSVConfiguration) throws {
        var data = data
        let strippedBOM = configuration.encoding == .utf8
            && data.starts(with: Data([0xEF, 0xBB, 0xBF]))
        if strippedBOM {
            data.removeFirst(3)
        }
        guard let text = String(data: data, encoding: configuration.encoding) else {
            throw CSVError.cannotDecodeData
        }
        self.init(text: text, configuration: configuration, hadUTF8BOM: strippedBOM)
    }

    /// Prepares a parser over an already decoded string.
    public init(string: String, configuration: CSVConfiguration) {
        self.init(text: string, configuration: configuration, hadUTF8BOM: false)
    }

    private init(text: String, configuration: CSVConfiguration, hadUTF8BOM: Bool) {
        scalars = Array(text.unicodeScalars)
        columnSeparator = configuration.columnSeparator.unicodeScalars.first
        quoteCharacter = configuration.quoteCharacter.unicodeScalars.first
        escapeCharacter = configuration.escapeCharacter.unicodeScalars.first
        self.hadUTF8BOM = hadUTF8BOM
    }

    /// Restarts reading from the beginning of the input.
    public mutating func reset() {
        index = 0
        hasReturnedRow = false
    }

    /// Returns the next row, or `nil` when the input is exhausted.
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
        return readUnquotedField()
    }

    private mutating func readQuotedField(quote: Unicode.Scalar) throws -> String {
        index += 1
        var field = String.UnicodeScalarView()

        while index < scalars.count {
            let scalar = scalars[index]
            guard scalar == quote else {
                field.append(scalar)
                index += 1
                continue
            }

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

    private mutating func readUnquotedField() -> String {
        var field = String.UnicodeScalarView()
        while index < scalars.count, !isFieldTerminator(scalars[index]) {
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
