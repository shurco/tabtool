import Foundation

/// Index-permutation helpers shared by row and column reordering.
public enum CSVReorder {
    /// Order that moves every index in `moving` by `offset` positions.
    ///
    /// Result position `i` holds the old index to place there. Selected
    /// indexes keep their relative gaps; the rest fill the free slots in
    /// ascending order. Returns `nil` for an empty selection or when any
    /// selected index would leave `0..<count`.
    public static func shifted(count: Int, moving: IndexSet, by offset: Int) -> [Int]? {
        guard !moving.isEmpty,
              moving.allSatisfy({ (0..<count).contains($0 + offset) })
        else { return nil }

        var result = [Int?](repeating: nil, count: count)
        for index in moving { result[index + offset] = index }
        var rest = (0..<count).filter { !moving.contains($0) }.makeIterator()
        return result.map { $0 ?? rest.next()! }
    }

    /// Whether `order` contains each of `0..<count` exactly once.
    static func isPermutation(_ order: [Int], count: Int) -> Bool {
        order.count == count && Set(order) == Set(0..<count)
    }
}
