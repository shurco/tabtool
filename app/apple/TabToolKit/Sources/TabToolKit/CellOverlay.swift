import Foundation

/// A cell address in the file: the physical row (header row included) and column.
public struct CellKey: Hashable, Sendable {
    public let row: Int
    public let column: Int

    public init(row: Int, column: Int) {
        self.row = row
        self.column = column
    }
}

/// The edited cells of a large file, kept apart from the file's bytes.
///
/// A reference type so a keystroke changes one entry instead of copying a
/// dictionary; `copy()` takes a snapshot when one is needed (for saving).
public final class CellOverlay: @unchecked Sendable {
    // The lock guards `cells`.
    private let lock = NSLock()
    private var cells: [CellKey: String] = [:]

    public init() {}

    public subscript(key: CellKey) -> String? {
        locked { cells[key] }
    }

    public var isEmpty: Bool { locked { cells.isEmpty } }
    public var count: Int { locked { cells.count } }

    public func copy() -> [CellKey: String] { locked { cells } }
    public func set(_ text: String, for key: CellKey) { locked { cells[key] = text } }
    public func remove(_ key: CellKey) { locked { _ = cells.removeValue(forKey: key) } }
    public func removeAll() { locked { cells.removeAll() } }

    private func locked<R>(_ body: () -> R) -> R {
        lock.lock()
        defer { lock.unlock() }
        return body()
    }
}
