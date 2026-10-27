import Foundation
@testable import TabToolKit

/// Streams a deterministic CSV of about `targetBytes` to `url` and returns the
/// number of rows (header included) and bytes written.
///
/// Even columns hold numbers, odd columns hold text (quoted when `quoted`).
/// Every `multilineEvery`-th row gets a quoted field with an embedded LF,
/// separator and doubled quotes. Rows end with CRLF when `crlf`, else LF.
@discardableResult
func generateLargeCSV(at url: URL, targetBytes: Int, columns: Int, quoted: Bool,
                      multilineEvery: Int?, crlf: Bool, bom: Bool,
                      seed: UInt64) throws -> (rows: Int, bytes: Int) {
    precondition(columns > 0)
    FileManager.default.createFile(atPath: url.path, contents: nil)
    let handle = try FileHandle(forWritingTo: url)
    defer { try? handle.close() }

    let chunkLimit = 8 << 20
    let terminator: [UInt8] = crlf ? [13, 10] : [10]
    var generator = SeededGenerator(seed: seed)
    var chunk: [UInt8] = []
    chunk.reserveCapacity(chunkLimit + 4096)
    var written = 0
    var rows = 0

    func flush() throws {
        try handle.write(contentsOf: chunk)
        written += chunk.count
        chunk.removeAll(keepingCapacity: true)
    }

    if bom { chunk.append(contentsOf: [0xEF, 0xBB, 0xBF]) }
    chunk.append(contentsOf: (0..<columns).map { "col\($0)" }.joined(separator: ",").utf8)
    chunk.append(contentsOf: terminator)
    rows += 1

    while written + chunk.count < targetBytes {
        for column in 0..<columns {
            if column > 0 { chunk.append(UInt8(ascii: ",")) }
            let value = generator.next()
            if let every = multilineEvery, rows % every == 0, column == min(1, columns - 1) {
                chunk.append(contentsOf: "\"line one\nline, \"\"two\"\" \(rows)\"".utf8)
            } else if column % 2 == 0 {
                chunk.append(contentsOf: "\(value % 1_000_000).\(column)".utf8)
            } else if quoted {
                chunk.append(contentsOf: "\"text\(value % 5000)\"".utf8)
            } else {
                chunk.append(contentsOf: "text\(value % 5000)".utf8)
            }
        }
        chunk.append(contentsOf: terminator)
        rows += 1
        if chunk.count >= chunkLimit { try flush() }
    }
    try flush()
    return (rows, written)
}

/// A value shared between the test thread and a background worker.
final class Locked<Value>: @unchecked Sendable {
    // The lock guards `value`.
    private let lock = NSLock()
    private var value: Value

    init(_ value: Value) { self.value = value }

    func withValue<R>(_ body: (inout Value) -> R) -> R {
        lock.lock()
        defer { lock.unlock() }
        return body(&value)
    }
}
