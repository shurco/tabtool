import Foundation

extension RowScanner {
    /// The byte range of every row in `bytes`, terminator included.
    ///
    /// `bytes` must start at a row start outside any quote, as an index block
    /// does. The ranges follow the same row rules as `scan`.
    static func rowRanges(in bytes: UnsafeRawBufferPointer,
                          configuration: CSVConfiguration) -> [Range<Int>] {
        var scanner = RowScanner(configuration: configuration)
        var index = RowIndex()
        var starts: [Int] = []
        for i in 0..<bytes.count {
            let rowsBefore = index.rowCount
            let startBefore = scanner.rowStart
            scanner.scanBytes(UnsafeRawBufferPointer(rebasing: bytes[i..<i + 1]), base: i, into: &index)
            if index.rowCount > rowsBefore { starts.append(startBefore) }
        }
        if starts.isEmpty { return [0..<0] }
        return starts.indices.map { starts[$0]..<(starts.indices.contains($0 + 1) ? starts[$0 + 1] : bytes.count) }
    }
}
