import Foundation

/// The decoded rows of one index block.
struct LargeBlock {
    var rows: [[String]]

    /// False when the bytes were not valid text or did not parse: the rows are
    /// then a best-effort view and must not be rewritten.
    var isFaithful: Bool

    /// Decodes the bytes of a block with `CSVParser`.
    ///
    /// Invalid bytes show as U+FFFD. A parse error shows each line as one cell.
    static func decode(_ bytes: Data, configuration: CSVConfiguration) -> LargeBlock {
        do {
            var parser = try CSVParser(data: bytes, configuration: configuration)
            return LargeBlock(rows: try parser.readAll(), isFaithful: true)
        } catch CSVError.cannotDecodeData {
            return LargeBlock(rows: parseLossy(bytes, configuration: configuration), isFaithful: false)
        } catch {
            return LargeBlock(rows: rawLines(lossyText(bytes, configuration: configuration)),
                              isFaithful: false)
        }
    }

    /// Text for display: U+FFFD for invalid UTF-8; other encodings fall back to
    /// Latin-1, which accepts every byte.
    private static func lossyText(_ bytes: Data, configuration: CSVConfiguration) -> String {
        if configuration.encoding == .utf8 {
            let bom = Data([0xEF, 0xBB, 0xBF])
            return String(decoding: bytes.starts(with: bom) ? bytes.dropFirst(3) : bytes[...],
                          as: UTF8.self)
        }
        return String(data: bytes, encoding: .isoLatin1) ?? ""
    }

    private static func parseLossy(_ bytes: Data, configuration: CSVConfiguration) -> [[String]] {
        var parser = CSVParser(string: lossyText(bytes, configuration: configuration),
                               configuration: configuration)
        return (try? parser.readAll()) ?? rawLines(lossyText(bytes, configuration: configuration))
    }

    private static func rawLines(_ text: String) -> [[String]] {
        var lines = text.split(omittingEmptySubsequences: false) { $0.isNewline }
            .map { [String($0)] }
        if lines.count > 1, lines.last == [""] { lines.removeLast() }
        return lines
    }
}
