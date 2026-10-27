import Foundation

/// Detects the format of a large file from its head and checks that the row
/// scanner can handle it.
///
/// The scanner finds rows by looking at single bytes, which is only sound for
/// ASCII-compatible encodings and ASCII delimiters. Everything else is refused
/// with a `CSVError` instead of being indexed wrongly.
public enum LargeFileSniffer {
    private static let scannableEncodings = Set([
        String.Encoding.utf8, .macOSRoman, .windowsCP1252, .isoLatin2, .windowsCP1250,
        .windowsCP1251, .windowsCP1253, .windowsCP1254, SupportedEncodings.gbk
    ].map(\.rawValue))

    /// Whether rows of a file in `encoding` can be found by scanning bytes.
    public static func isByteScannable(_ encoding: String.Encoding) -> Bool {
        scannableEncodings.contains(encoding.rawValue)
    }

    /// Throws when the scanner cannot handle `configuration`: a refused
    /// encoding, or a separator, quote or escape that is not one ASCII character.
    public static func validate(_ configuration: CSVConfiguration) throws {
        guard isByteScannable(configuration.encoding) else {
            throw CSVError.unsupportedLargeFileEncoding
        }
        let delimiters = [configuration.columnSeparator, configuration.quoteCharacter,
                          configuration.escapeCharacter]
        guard delimiters.allSatisfy({ $0.utf8.count <= 1 && $0.utf8.allSatisfy { $0 < 0x80 } }) else {
            throw CSVError.unsupportedLargeFileDelimiter
        }
    }

    /// Detects the configuration from the first `LargeFile.sampleBytes` bytes.
    ///
    /// The sample ends at the last row break outside quotes, so no row is cut
    /// in half. It must parse strictly; the parser's error is thrown otherwise.
    /// A single row longer than the sample is parsed as is (ponytail: such a
    /// file is reported as malformed; widen the sample if that ever matters).
    public static func sniff(_ file: MappedFile, prefersChineseEncoding: Bool = false) throws
        -> CSVConfiguration {
        let sample = try sample(of: file)
        let detected = CSVFormatDetector(data: sample,
                                         prefersChineseEncoding: prefersChineseEncoding).detect()
        try verify(sample, as: detected)
        return detected
    }

    /// Checks that `configuration` suits the file: scannable, and the head of
    /// the file parses strictly with it.
    public static func check(_ file: MappedFile, configuration: CSVConfiguration) throws {
        try verify(try sample(of: file), as: configuration)
    }

    private static func sample(of file: MappedFile) throws -> Data {
        let head = file.withBytes(0..<min(file.size, LargeFile.sampleBytes)) { Data($0) }
        guard !looksLikeRefusedEncoding(head) else { throw CSVError.unsupportedLargeFileEncoding }
        return file.size > head.count ? Data(head.prefix(safeCut(head))) : head
    }

    private static func verify(_ sample: Data, as configuration: CSVConfiguration) throws {
        try validate(configuration)
        var parser = try CSVParser(data: sample, configuration: configuration)
        _ = try parser.readAll()
    }

    /// UTF-16/32 (byte order mark or NUL bytes) and ISO-2022-JP (escape sequences).
    private static func looksLikeRefusedEncoding(_ bytes: Data) -> Bool {
        let marks: [[UInt8]] = [[0xFF, 0xFE], [0xFE, 0xFF], [0x00, 0x00, 0xFE, 0xFF]]
        if marks.contains(where: { bytes.starts(with: $0) }) || bytes.contains(0) { return true }
        return zip(bytes, bytes.dropFirst()).contains {
            $0 == 0x1B && ($1 == UInt8(ascii: "$") || $1 == UInt8(ascii: "("))
        }
    }

    /// The length of `bytes` up to the last row break with an even number of
    /// quotes before it, or all of it when there is none.
    private static func safeCut(_ bytes: Data) -> Int {
        var quotes = 0
        var cut = bytes.count
        var found = false
        for (offset, byte) in bytes.enumerated() {
            if byte == UInt8(ascii: "\"") {
                quotes += 1
            } else if (byte == 0x0A || byte == 0x0D) && quotes.isMultiple(of: 2) {
                cut = offset + 1
                found = true
            }
        }
        return found ? cut : bytes.count
    }
}
