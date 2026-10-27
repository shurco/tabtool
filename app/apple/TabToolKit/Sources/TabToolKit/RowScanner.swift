import Foundation

/// Finds row boundaries in raw bytes without decoding them.
///
/// Rows end at LF, CR or CRLF outside quotes; a final terminator adds no row
/// and an empty input is one row. Scanning is resumable: quote state and a
/// pending CR carry across calls, so a file can be fed in any chunking.
///
/// The scanner looks only at the bytes of the separator, quote and escape
/// characters, so it supports ASCII-compatible encodings and single-byte
/// delimiters. Quote handling is lenient: it follows quote parity (or the
/// dedicated escape) and never throws, so malformed input yields a best-effort
/// index instead of an error.
public struct RowScanner {
    private static let lineFeed = UInt8(ascii: "\n")
    private static let carriageReturn = UInt8(ascii: "\r")

    private let separator: UInt8?
    private let quote: UInt8?
    private let escape: UInt8?

    private var inQuote = false
    private var escaped = false
    private var afterCarriageReturn = false
    private var atRowStart = true
    private var rowStart = 0
    private var separators = 0

    public init(configuration: CSVConfiguration) {
        separator = Self.singleByte(configuration.columnSeparator)
        quote = Self.singleByte(configuration.quoteCharacter)
        let escape = Self.singleByte(configuration.escapeCharacter)
        self.escape = escape == quote ? nil : escape
    }

    private static func singleByte(_ text: String) -> UInt8? {
        text.utf8.count == 1 ? text.utf8.first : nil
    }

    /// Scans `bytes`, which start at absolute offset `base`, into `index`.
    ///
    /// Chunks must be contiguous. Pass `final: true` with the last chunk.
    public mutating func scan(_ bytes: UnsafeRawBufferPointer, base: Int,
                              into index: inout RowIndex, final: Bool) {
        scanBytes(bytes, base: base, into: &index)
        finishChunk(end: base + bytes.count, into: &index, final: final)
    }

    mutating func scanBytes(_ bytes: UnsafeRawBufferPointer, base: Int,
                            into index: inout RowIndex) {
        for i in 0..<bytes.count {
            let byte = bytes[i]
            if afterCarriageReturn {
                afterCarriageReturn = false
                if byte == Self.lineFeed {
                    rowStart = base + i + 1
                    continue
                }
            }
            if atRowStart { beginRow(into: &index) }
            if inQuote {
                if escaped {
                    escaped = false
                } else if byte == escape {
                    escaped = true
                } else if byte == quote {
                    inQuote = false
                }
            } else if byte == quote {
                inQuote = true
            } else if byte == separator {
                separators += 1
            } else if byte == Self.lineFeed {
                endRow(at: base + i + 1, into: &index)
            } else if byte == Self.carriageReturn {
                endRow(at: base + i + 1, into: &index)
                afterCarriageReturn = true
            }
        }
    }

    mutating func finishChunk(end: Int, into index: inout RowIndex, final: Bool) {
        if final && index.rowCount == 0 && atRowStart {
            beginRow(into: &index)
        }
        if !atRowStart {
            index.maxFieldCount = max(index.maxFieldCount, separators + 1)
        }
        index.scannedBytes = end
        index.endsInsideQuote = inQuote
    }

    mutating func beginRow(into index: inout RowIndex) {
        if index.rowCount % LargeFile.blockRows == 0 {
            index.checkpoints.append(UInt64(rowStart))
        }
        index.rowCount += 1
        atRowStart = false
        separators = 0
    }

    mutating func endRow(at nextStart: Int, into index: inout RowIndex) {
        index.maxFieldCount = max(index.maxFieldCount, separators + 1)
        rowStart = nextStart
        atRowStart = true
    }
}
