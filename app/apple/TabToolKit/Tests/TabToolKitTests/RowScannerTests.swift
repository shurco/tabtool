import Foundation
import Testing
@testable import TabToolKit

/// Scans `bytes` in chunks cut at `cuts` and returns the final index.
func scanIndex(_ bytes: [UInt8], configuration: CSVConfiguration = .init(),
               cuts: [Int] = []) -> RowIndex {
    var scanner = RowScanner(configuration: configuration)
    var index = RowIndex()
    let bounds = [0] + cuts + [bytes.count]
    for (offset, pair) in zip(bounds, bounds.dropFirst()).enumerated() {
        let isLast = offset == bounds.count - 2
        bytes[pair.0..<pair.1].withUnsafeBytes {
            scanner.scan($0, base: pair.0, into: &index, final: isLast)
        }
    }
    return index
}

/// Asserts the index agrees with `CSVParser`, including each checkpoint's row.
func expectIndexMatchesParser(_ bytes: [UInt8], _ index: RowIndex,
                              configuration: CSVConfiguration = .init(),
                              sourceLocation: SourceLocation = #_sourceLocation) throws {
    var parser = try CSVParser(data: Data(bytes), configuration: configuration)
    let rows = try parser.readAll()
    #expect(index.rowCount == rows.count, sourceLocation: sourceLocation)
    #expect(index.maxFieldCount == rows.map(\.count).max(), sourceLocation: sourceLocation)
    #expect(index.checkpoints.count == (rows.count + 63) / 64, sourceLocation: sourceLocation)
    for (block, offset) in index.checkpoints.enumerated() {
        var tail = try CSVParser(data: Data(bytes[Int(offset)...]), configuration: configuration)
        #expect(try tail.nextRow() == rows[block * 64], sourceLocation: sourceLocation)
    }
}

private let edgeCases = [
    "", "a", "a\n", "a\n\n", "\n", "a\r\nb", "a\rb\r", "a\r\n", "\r\n\r\n", "\r\r\n\n",
    "a,b,\n", ",", "\"x\ny\",z\nq", "\"a\"\"b\"\n1", "\"\"", "a,\"b\r\nc\"\r\nd",
    "\n\n\n", "x\r\ny\rz\nw",
]

@Suite struct RowScannerTests {
    @Test(arguments: edgeCases)
    func matchesParserOnEdgeCases(_ text: String) throws {
        let bytes = Array(text.utf8)
        try expectIndexMatchesParser(bytes, scanIndex(bytes))
    }

    @Test(arguments: edgeCases)
    func resumesAcrossChunks(_ text: String) {
        let bytes = Array(text.utf8)
        let whole = scanIndex(bytes)
        for cut in 0...bytes.count {
            #expect(scanIndex(bytes, cuts: [cut]) == whole)
        }
        #expect(scanIndex(bytes, cuts: Array(0..<bytes.count)) == whole)
    }

    @Test func checkpointsAtMultiplesOf64() throws {
        let bytes = Array((0..<200).map { "r\($0)\n" }.joined().utf8)
        let index = scanIndex(bytes)
        #expect(index.rowCount == 200)
        #expect(index.checkpoints.count == 4)
        #expect(index.blockRange(0, fileSize: bytes.count)?.lowerBound == 0)
        #expect(index.blockRange(4, fileSize: bytes.count) == nil)
        let last = try #require(index.blockRange(3, fileSize: bytes.count))
        #expect(last.upperBound == bytes.count)
        try expectIndexMatchesParser(bytes, index)
    }

    @Test func noCheckpointForRowThatNeverStarts() {
        let bytes = Array((0..<64).map { "r\($0)\n" }.joined().utf8)
        let index = scanIndex(bytes)
        #expect(index.rowCount == 64)
        #expect(index.checkpoints.count == 1)
    }

    @Test func backslashEscapeKeepsEscapedQuote() throws {
        let configuration = CSVConfiguration(escapeCharacter: "\\")
        for text in ["\"a\\\"b\",c\nd", "\"a\\\\\",x\ny", "\"a\\\n\",x\ny"] {
            let bytes = Array(text.utf8)
            let index = scanIndex(bytes, configuration: configuration)
            try expectIndexMatchesParser(bytes, index, configuration: configuration)
            #expect(index.rowCount == 2)
        }
    }

    @Test func quotelessConfigurationIgnoresQuotes() {
        let configuration = CSVConfiguration(quoteCharacter: "")
        let index = scanIndex(Array("a\"b\nc".utf8), configuration: configuration)
        #expect(index.rowCount == 2)
        #expect(!index.endsInsideQuote)
    }

    @Test func tabSeparatorCountsFields() {
        let index = scanIndex(Array("a\tb\tc\nd".utf8), configuration: .init(columnSeparator: "\t"))
        #expect(index.maxFieldCount == 3)
    }

    @Test func unbalancedQuoteSetsFlag() {
        let index = scanIndex(Array("a\n\"b\nc\n".utf8))
        #expect(index.endsInsideQuote)
        #expect(index.rowCount == 2)
    }

    @Test(arguments: 1...200)
    func fuzzMatchesParser(_ seed: Int) throws {
        let bytes = RowScannerFuzz.makeInput(seed: UInt64(seed))
        let whole = scanIndex(bytes)
        try expectIndexMatchesParser(bytes, whole)
        var generator = SeededGenerator(seed: UInt64(seed))
        let cuts = (0..<3).map { _ in Int(generator.next() % UInt64(bytes.count + 1)) }.sorted()
        #expect(scanIndex(bytes, cuts: cuts) == whole)
    }
}

/// Random CSV bytes with mixed terminators, quoting and multibyte text.
enum RowScannerFuzz {
    static func makeInput(seed: UInt64, rows: Int = 150) -> [UInt8] {
        var generator = SeededGenerator(seed: seed)
        let plain = ["a", "bb", "", "é", "日本", "12.5", "x y"]
        let special = [",", "\n", "\r\n", "\r", "\"", "\"\"", "q,\n\"z"]
        var text = ""
        for row in 0..<rows {
            let fieldCount = Int(generator.next() % 5) + 1
            let fields = (0..<fieldCount).map { _ -> String in
                let value = plain.randomElement(using: &generator)!
                guard generator.next() % 4 == 0 else { return value }
                let extra = special.randomElement(using: &generator)!
                return "\"" + (value + extra).replacingOccurrences(of: "\"", with: "\"\"") + "\""
            }
            text += fields.joined(separator: ",")
            if row < rows - 1 || generator.next() % 2 == 0 {
                text += ["\n", "\r\n", "\r"].randomElement(using: &generator)!
            }
        }
        return Array(text.utf8)
    }
}
