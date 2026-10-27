import Foundation

/// Everything a streaming save needs, captured at one moment.
public struct LargeSaveSnapshot: Sendable {
    public let file: MappedFile
    public let index: RowIndex
    public let configuration: CSVConfiguration
    public let hadBOM: Bool
    /// The edited cells, keyed by physical row.
    public let edits: [CellKey: String]
}

/// Why a large file cannot be saved right now.
public enum LargeFileSaveError: Error, Equatable, Sendable {
    case indexingNotFinished
    /// The file on disk is no longer the one that was opened.
    case sourceChanged
    case insufficientDiskSpace
}

/// Rewrites a large file with its edited cells, streaming through the mapping.
///
/// Rows without edits are copied byte for byte. An edited row is parsed from
/// its block, patched, and serialized again with `CSVWriter` (every field
/// quoted when quoting is on), keeping its original line terminator and, for
/// row 0, the UTF-8 BOM. Memory use is one block plus the copy chunk.
public enum LargeFileSaver {
    private static let utf8BOM = Data([0xEF, 0xBB, 0xBF])

    /// Writes the saved file to `url` and returns its size in bytes.
    ///
    /// Reports the fraction done through `progress`. Stops with
    /// `CancellationError` when the current task is cancelled. A failed or
    /// cancelled write leaves no file at `url`.
    @discardableResult
    public static func write(_ snapshot: LargeSaveSnapshot, to url: URL, bufferBytes: Int = 4 << 20,
                             progress: @Sendable (Double) -> Void) throws -> Int {
        let file = snapshot.file
        guard snapshot.index.scannedBytes >= file.size else { throw LargeFileSaveError.indexingNotFinished }
        guard file.isStale() == .unchanged else { throw LargeFileSaveError.sourceChanged }
        // 0666 less the umask, like any new document; Save As keeps this file.
        let descriptor = open(url.path, O_WRONLY | O_CREAT | O_TRUNC, 0o666)
        guard descriptor >= 0 else { throw CocoaError(.fileWriteUnknown) }
        defer { close(descriptor) }
        do {
            return try stream(snapshot, to: descriptor, bufferBytes: max(bufferBytes, 1), progress: progress)
        } catch {
            unlink(url.path)
            throw error
        }
    }

    /// Writes the saved file next to `url` in a replacement directory and swaps it in.
    ///
    /// Needs twice the file's size free on the volume; `availableCapacity`
    /// returns the free bytes (`nil` skips the check).
    public static func save(_ snapshot: LargeSaveSnapshot, replacing url: URL,
                            availableCapacity: (URL) -> Int64? = volumeCapacity,
                            progress: @Sendable (Double) -> Void) throws {
        guard snapshot.index.scannedBytes >= snapshot.file.size else {
            throw LargeFileSaveError.indexingNotFinished
        }
        if let free = availableCapacity(url), free < Int64(snapshot.file.size) * 2 {
            throw LargeFileSaveError.insufficientDiskSpace
        }
        let directory = try FileManager.default.url(for: .itemReplacementDirectory, in: .userDomainMask,
                                                    appropriateFor: url, create: true)
        defer { try? FileManager.default.removeItem(at: directory) }
        let temporary = directory.appendingPathComponent(url.lastPathComponent)
        try write(snapshot, to: temporary, progress: progress)
        _ = try FileManager.default.replaceItemAt(url, withItemAt: temporary)
    }

    /// Free bytes on the volume holding `url`, counting purgeable space.
    public static func volumeCapacity(_ url: URL) -> Int64? {
        try? url.resourceValues(forKeys: [.volumeAvailableCapacityForImportantUsageKey])
            .volumeAvailableCapacityForImportantUsage
    }

    // MARK: Streaming

    private static func stream(_ snapshot: LargeSaveSnapshot, to descriptor: Int32, bufferBytes: Int,
                               progress: @Sendable (Double) -> Void) throws -> Int {
        let file = snapshot.file
        var cursor = 0
        var written = 0
        var buffer: [UInt8] = []

        func emit(_ data: Data) throws {
            try data.withUnsafeBytes { try writeAll($0, to: descriptor) }
            written += data.count
        }
        func copy(upTo end: Int) throws {
            while cursor < end {
                try Task.checkCancellation()
                let range = cursor..<min(cursor + bufferBytes, end)
                try file.read(range, into: &buffer) { bytes in
                    guard bytes.count == range.count else { throw LargeFileSaveError.sourceChanged }
                    try writeAll(bytes, to: descriptor)
                }
                written += range.count
                cursor = range.upperBound
                progress(Double(cursor) / Double(max(file.size, 1)))
            }
        }

        let rowsByBlock = Dictionary(grouping: snapshot.edits.keys, by: { $0.row / LargeFile.blockRows })
        for blockNumber in rowsByBlock.keys.sorted() {
            guard let blockRange = snapshot.index.blockRange(blockNumber, fileSize: file.size) else {
                throw CSVError.rowNotEditable
            }
            let bytes = file.withBytes(blockRange) { Data($0) }
            let block = LargeBlock.decode(bytes, configuration: snapshot.configuration)
            let ranges = bytes.withUnsafeBytes {
                RowScanner.rowRanges(in: $0, configuration: snapshot.configuration)
            }
            guard block.isFaithful, ranges.count == block.rows.count else { throw CSVError.rowNotEditable }
            for row in Set(rowsByBlock[blockNumber]!.map(\.row)).sorted() {
                let local = row % LargeFile.blockRows
                guard local < ranges.count else { throw CSVError.rowNotEditable }
                let range = ranges[local]
                try copy(upTo: blockRange.lowerBound + range.lowerBound)
                let terminator = bytes[range].suffix(while: { $0 == 10 || $0 == 13 })
                try emit(serialize(block.rows[local], row: row, snapshot: snapshot, terminator: terminator))
                cursor = blockRange.lowerBound + range.upperBound
            }
        }
        try copy(upTo: file.size)
        progress(1)
        return written
    }

    /// The edited row as bytes: patched fields, then the original terminator.
    private static func serialize(_ fields: [String], row: Int, snapshot: LargeSaveSnapshot,
                                  terminator: Data) throws -> Data {
        let edited = snapshot.edits.filter { $0.key.row == row }
        var patched = fields
        let width = max(fields.count, (edited.keys.map(\.column).max() ?? -1) + 1)
        patched += Array(repeating: "", count: width - fields.count)
        for (key, text) in edited { patched[key.column] = text }
        var data = row == 0 && snapshot.hadBOM ? utf8BOM : Data()
        data += try CSVWriter(table: CSVTable(rows: [patched], headers: nil), header: nil,
                              configuration: snapshot.configuration).data()
        return data + terminator
    }

    private static func writeAll(_ buffer: UnsafeRawBufferPointer, to descriptor: Int32) throws {
        var offset = 0
        while offset < buffer.count {
            let count = Foundation.write(descriptor, buffer.baseAddress! + offset, buffer.count - offset)
            if count < 0 {
                if errno == EINTR { continue }
                throw CocoaError(errno == ENOSPC ? .fileWriteOutOfSpace : .fileWriteUnknown)
            }
            offset += count
        }
    }
}

private extension Data {
    /// The trailing bytes that satisfy `predicate`.
    func suffix(while predicate: (UInt8) -> Bool) -> Data {
        var start = endIndex
        while start > startIndex, predicate(self[start - 1]) { start -= 1 }
        return Data(self[start...])
    }
}
