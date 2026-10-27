import Foundation

/// A sparse index of row starts: one byte offset per `LargeFile.blockRows` rows.
///
/// Block `b` covers the rows `64 * b ..< 64 * (b + 1)`. The index costs
/// 0.125 bytes per row, so a billion rows stay near 125 MB.
public struct RowIndex: Sendable, Equatable {
    /// Byte offset of the first row of every block.
    public internal(set) var checkpoints: [UInt64] = []

    /// Rows that have started in the scanned bytes.
    public internal(set) var rowCount = 0

    /// The most fields seen in one row, counting separators outside quotes.
    public internal(set) var maxFieldCount = 0

    /// Bytes consumed so far.
    public internal(set) var scannedBytes = 0

    /// Whether the scanned bytes end inside an open quoted field.
    public internal(set) var endsInsideQuote = false

    public init() {}

    /// Appends the checkpoints of `next`, a scan that continued this one, and
    /// takes over its counters.
    mutating func extend(with next: RowIndex) {
        checkpoints.append(contentsOf: next.checkpoints)
        rowCount = next.rowCount
        maxFieldCount = next.maxFieldCount
        scannedBytes = next.scannedBytes
        endsInsideQuote = next.endsInsideQuote
    }

    /// The byte range of `block`, or `nil` when the block is not indexed.
    ///
    /// The last block ends at `fileSize`.
    public func blockRange(_ block: Int, fileSize: Int) -> Range<Int>? {
        guard checkpoints.indices.contains(block) else { return nil }
        let start = Int(checkpoints[block])
        let end = block + 1 < checkpoints.count ? Int(checkpoints[block + 1]) : fileSize
        return start..<max(start, end)
    }
}
