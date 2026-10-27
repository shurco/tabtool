import Foundation

/// A `RowSource` over a memory-mapped file.
///
/// Only the 64-row block holding a requested cell is copied out of the mapping
/// and parsed (with `CSVParser`), then kept in a small LRU cache. Edited cells
/// live in the `overlay` and win over the file's bytes. Rows still being
/// indexed are hidden until their end is known.
public final class LargeCSVSource: RowSource, @unchecked Sendable {
    private static let cacheCapacity = 64
    private static let utf8BOM: [UInt8] = [0xEF, 0xBB, 0xBF]

    /// The format the file was opened with. `firstRowAsHeader` is its value at
    /// creation; `setHeaderEnabled` changes the live setting.
    public let configuration: CSVConfiguration
    public let overlay: CellOverlay
    public let hadUTF8BOM: Bool
    private let file: MappedFile
    private let indexer: LargeFileIndexer
    private var headerRows: Int

    private struct CachedBlock {
        let block: LargeBlock
        let end: Int
        var lastUse: Int
    }

    // The lock guards the cache, its counters, the column ids and the header
    // row count; blocks are decoded outside it.
    private let lock = NSLock()
    private var cache: [Int: CachedBlock] = [:]
    private var useCounter = 0
    private var ids: [UUID] = []
    private(set) var cacheHits = 0
    private(set) var cacheMisses = 0

    /// `columnIDs` carries the column identities of a source this one replaces
    /// (the same file after a save), so the grid keeps its columns.
    public init(file: MappedFile, configuration: CSVConfiguration, indexer: LargeFileIndexer,
                overlay: CellOverlay = CellOverlay(), columnIDs: [UUID] = []) {
        self.file = file
        self.configuration = configuration
        self.indexer = indexer
        self.overlay = overlay
        ids = columnIDs
        headerRows = configuration.firstRowAsHeader ? 1 : 0
        hadUTF8BOM = configuration.encoding == .utf8
            && file.withBytes(0..<3) { Array($0) } == Self.utf8BOM
    }

    deinit {
        indexer.cancel()
    }

    // MARK: Indexing

    /// Whether the whole file has been indexed.
    public var isIndexed: Bool { indexer.isFinished }

    /// The share of the file scanned so far, from 0 to 1.
    public var indexingFraction: Double {
        file.size == 0 ? 1 : min(1, Double(indexer.snapshot.scannedBytes) / Double(file.size))
    }

    // MARK: RowSource

    private var headerOffset: Int { locked { headerRows } }

    /// Treats the first row as the header, or as data again. The file's bytes
    /// and the edits are untouched; only the mapping from rows to file rows moves.
    public func setHeaderEnabled(_ enabled: Bool) {
        locked { headerRows = enabled ? 1 : 0 }
    }

    public var rowCount: Int {
        let index = indexer.snapshot
        return max(0, visibleRows(index) - headerOffset)
    }

    public var columnCount: Int {
        min(indexer.snapshot.maxFieldCount, LargeFile.maxColumns)
    }

    public var columnIDs: [UUID] {
        let count = columnCount
        return locked {
            while ids.count < count { ids.append(UUID()) }
            return Array(ids.prefix(count))
        }
    }

    public var headers: [String]? {
        guard headerOffset == 1, let row = block(0)?.block.rows.first else { return nil }
        let count = columnCount
        return (0..<count).map { $0 < row.count ? row[$0] : "" }
    }

    public func cell(row: Int, column: Int) -> String {
        guard row >= 0, row < rowCount, column >= 0, column < columnCount else { return "" }
        let physical = row + headerOffset
        if let edited = overlay[CellKey(row: physical, column: column)] { return edited }
        return baseCell(physical: physical, column: column)
    }

    // MARK: Editing

    public var staleness: Staleness { file.isStale() }

    /// The file row behind a displayed row.
    public func physicalRow(forDisplay row: Int) -> Int { row + headerOffset }

    /// Whether `row` can be edited: indexing is done, the file is unchanged and
    /// the row's bytes were read faithfully.
    public func isEditable(row: Int) -> Bool {
        let index = indexer.snapshot
        guard index.scannedBytes >= file.size, row >= 0, row < rowCount,
              file.isStale() == .unchanged,
              let cached = block((row + headerOffset) / LargeFile.blockRows) else { return false }
        return cached.block.isFaithful
    }

    /// Sets a cell and returns the previous override, or `nil` when there was none.
    ///
    /// Setting the file's own text removes the override. Throws
    /// `CSVError.rowNotEditable`, `.ambiguousOutput` or `.unencodableText`.
    @discardableResult
    public func setCell(_ text: String, row: Int, column: Int) throws -> String? {
        guard column >= 0, column < columnCount, isEditable(row: row) else {
            throw CSVError.rowNotEditable
        }
        try validate(text)
        let physical = row + headerOffset
        let key = CellKey(row: physical, column: column)
        let previous = overlay[key]
        if text == baseCell(physical: physical, column: column) {
            overlay.remove(key)
        } else {
            overlay.set(text, for: key)
        }
        return previous
    }

    /// The state a streaming save needs: the file, its index and the edits so far.
    public func saveSnapshot() -> LargeSaveSnapshot {
        LargeSaveSnapshot(file: file, index: indexer.snapshot, configuration: configuration,
                          hadBOM: hadUTF8BOM, edits: overlay.copy())
    }

    private func validate(_ text: String) throws {
        let table = CSVTable(rows: [[text]], headers: nil)
        switch CSVWriteValidation.issue(table: table, header: nil, configuration: configuration) {
        case .ambiguousOutput: throw CSVError.ambiguousOutput
        case .unencodableText: throw CSVError.unencodableText
        case nil: break
        }
    }

    // MARK: Blocks

    /// Rows whose end is known: all of them once indexing is done, else all but
    /// the one still being scanned.
    private func visibleRows(_ index: RowIndex) -> Int {
        index.scannedBytes >= file.size ? index.rowCount : max(0, index.rowCount - 1)
    }

    private func baseCell(physical: Int, column: Int) -> String {
        guard let cached = block(physical / LargeFile.blockRows) else { return "" }
        let rows = cached.block.rows
        let rowInBlock = physical % LargeFile.blockRows
        guard rowInBlock < rows.count, column < rows[rowInBlock].count else { return "" }
        return rows[rowInBlock][column]
    }

    /// The decoded block, from the cache or the file; `nil` when it is not
    /// indexed or the mapped file was truncated or rewritten. A file replaced
    /// at the path (a save) still reads from the mapping.
    private func block(_ number: Int) -> CachedBlock? {
        let index = indexer.snapshot
        guard let range = index.blockRange(number, fileSize: min(index.scannedBytes, file.size)) else { return nil }
        if let hit = lookup(number, end: range.upperBound) { return hit }
        guard file.isReadable else { return nil }
        let bytes = file.withBytes(range) { Data($0) }
        let decoded = LargeBlock.decode(bytes, configuration: configuration)
        return store(number, decoded, end: range.upperBound)
    }

    private func lookup(_ number: Int, end: Int) -> CachedBlock? {
        locked {
            guard var entry = cache[number], entry.end == end else {
                cacheMisses += 1
                return nil
            }
            cacheHits += 1
            useCounter += 1
            entry.lastUse = useCounter
            cache[number] = entry
            return entry
        }
    }

    private func store(_ number: Int, _ block: LargeBlock, end: Int) -> CachedBlock {
        locked {
            useCounter += 1
            let entry = CachedBlock(block: block, end: end, lastUse: useCounter)
            cache[number] = entry
            if cache.count > Self.cacheCapacity,
               let oldest = cache.min(by: { $0.value.lastUse < $1.value.lastUse })?.key {
                cache.removeValue(forKey: oldest)
            }
            return entry
        }
    }

    private func locked<R>(_ body: () -> R) -> R {
        lock.lock()
        defer { lock.unlock() }
        return body()
    }
}
