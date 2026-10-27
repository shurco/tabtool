import Foundation
import Testing
@testable import TabToolKit

private func makeSource(_ file: MappedFile, _ configuration: CSVConfiguration = CSVConfiguration()
) -> LargeCSVSource {
    let indexer = LargeFileIndexer(file: file, configuration: configuration, stepBytes: 4096,
                                   progressInterval: 0)
    indexer.start { _ in }
    indexer.wait()
    return LargeCSVSource(file: file, configuration: configuration, indexer: indexer)
}

private func textSource(_ text: String, _ dir: TempDirectory,
                        _ configuration: CSVConfiguration = CSVConfiguration()) throws
    -> LargeCSVSource {
    makeSource(try MappedFile(url: dir.file("t.csv", Array(text.utf8))), configuration)
}

@Suite struct LargeCSVSourceTests {
    @Test func cellsEqualInMemoryTable() throws {
        let dir = TempDirectory()
        let url = dir.url.appendingPathComponent("big.csv")
        let generated = try generateLargeCSV(at: url, targetBytes: 300_000, columns: 5,
                                             quoted: true, multilineEvery: 7, crlf: false,
                                             bom: false, seed: 5)
        var parser = try CSVParser(data: Data(contentsOf: url), configuration: CSVConfiguration())
        let table = CSVTable(rows: try parser.readAll(), headers: nil)
        let source = makeSource(try MappedFile(url: url))
        #expect(source.rowCount == generated.rows)
        #expect(source.rowCount == table.rowCount)
        #expect(source.columnCount == table.columnCount)
        for row in 0..<table.rowCount {
            for column in 0..<table.columnCount {
                #expect(source.cell(row: row, column: column) == table.cell(row: row, column: column))
            }
        }
    }

    @Test func headerOffset() throws {
        let dir = TempDirectory()
        let source = try textSource("a,b\n1,2\n3,4\n", dir, CSVConfiguration(firstRowAsHeader: true))
        #expect(source.rowCount == 2)
        #expect(source.headers == ["a", "b"])
        #expect(source.cell(row: 0, column: 0) == "1")
        #expect(source.physicalRow(forDisplay: 0) == 1)
    }

    @Test func noHeaderMeansNilHeaders() throws {
        let dir = TempDirectory()
        let source = try textSource("a,b\n", dir)
        #expect(source.headers == nil)
    }

    @Test func raggedRowPadsWithEmpty() throws {
        let dir = TempDirectory()
        let source = try textSource("a,b,c\nx\n", dir)
        #expect(source.columnCount == 3)
        #expect(source.cell(row: 1, column: 0) == "x")
        #expect(source.cell(row: 1, column: 2) == "")
        #expect(source.cell(row: 9, column: 0) == "")
        #expect(source.cell(row: 0, column: 9) == "")
    }

    @Test func columnCountCappedAt2048() throws {
        let dir = TempDirectory()
        let source = try textSource(Array(repeating: "x", count: 2100).joined(separator: ",") + "\n", dir)
        #expect(source.columnCount == LargeFile.maxColumns)
        #expect(source.columnIDs.count == LargeFile.maxColumns)
    }

    @Test func columnIDsAreStable() throws {
        let dir = TempDirectory()
        let source = try textSource("a,b\n", dir)
        #expect(source.columnIDs == source.columnIDs)
        #expect(Set(source.columnIDs).count == 2)
    }

    @Test func columnIDsCarryOverToAReplacingSource() throws {
        let dir = TempDirectory()
        let first = try textSource("a,b\n", dir)
        let file = try MappedFile(url: dir.file("u.csv", Array("a,b,c\n".utf8)))
        let indexer = LargeFileIndexer(file: file, configuration: CSVConfiguration())
        indexer.start { _ in }
        indexer.wait()
        let second = LargeCSVSource(file: file, configuration: CSVConfiguration(), indexer: indexer,
                                    columnIDs: first.columnIDs)
        #expect(Array(second.columnIDs.prefix(2)) == first.columnIDs)
        #expect(second.columnIDs.count == 3)
    }

    @Test func bomFileRow0() throws {
        let dir = TempDirectory()
        let file = try MappedFile(url: dir.file("b.csv", [0xEF, 0xBB, 0xBF] + Array("a,b\n".utf8)))
        let source = makeSource(file)
        #expect(source.hadUTF8BOM)
        #expect(source.cell(row: 0, column: 0) == "a")
    }

    @Test func lossyBlockRejectsEdit() throws {
        let dir = TempDirectory()
        let file = try MappedFile(url: dir.file("l.csv", Array("a,b\n".utf8) + [0xFF, 0x2C, 0x78, 0x0A]))
        let source = makeSource(file)
        #expect(source.cell(row: 1, column: 0) == "\u{FFFD}")
        #expect(!source.isEditable(row: 1))
        #expect(throws: CSVError.rowNotEditable) { try source.setCell("z", row: 1, column: 0) }
    }

    @Test func setCellOverrideAndRevert() throws {
        let dir = TempDirectory()
        let source = try textSource("a,b\n1,2\n", dir)
        #expect(try source.setCell("new", row: 1, column: 1) == nil)
        #expect(source.cell(row: 1, column: 1) == "new")
        #expect(source.overlay.count == 1)
        #expect(try source.setCell("again", row: 1, column: 1) == "new")
        #expect(try source.setCell("2", row: 1, column: 1) == "again")
        #expect(source.overlay.isEmpty)
        #expect(source.cell(row: 1, column: 1) == "2")
    }

    @Test func editedCellKeyUsesPhysicalRow() throws {
        let dir = TempDirectory()
        let source = try textSource("a,b\n1,2\n", dir, CSVConfiguration(firstRowAsHeader: true))
        _ = try source.setCell("new", row: 0, column: 0)
        #expect(source.overlay[CellKey(row: 1, column: 0)] == "new")
    }

    @Test func setCellRejectsAmbiguousWhenQuotesOff() throws {
        let dir = TempDirectory()
        let source = try textSource("a;b\n1;2\n", dir,
                                    CSVConfiguration(columnSeparator: ";", quoteCharacter: ""))
        #expect(throws: CSVError.ambiguousOutput) { try source.setCell("x;y", row: 0, column: 0) }
        #expect(source.overlay.isEmpty)
    }

    @Test func setCellRejectsOutOfRange() throws {
        let dir = TempDirectory()
        let source = try textSource("a,b\n", dir)
        #expect(throws: CSVError.rowNotEditable) { try source.setCell("x", row: 5, column: 0) }
        #expect(throws: CSVError.rowNotEditable) { try source.setCell("x", row: 0, column: 5) }
    }

    @Test func setCellRejectedWhileIndexing() throws {
        let dir = TempDirectory()
        let url = dir.url.appendingPathComponent("big.csv")
        try generateLargeCSV(at: url, targetBytes: 300_000, columns: 3, quoted: false,
                             multilineEvery: nil, crlf: false, bom: false, seed: 1)
        let file = try MappedFile(url: url)
        let indexer = LargeFileIndexer(file: file, configuration: CSVConfiguration(),
                                       stepBytes: 4096, progressInterval: 0)
        indexer.start { _ in indexer.cancel() }
        indexer.wait()
        let source = LargeCSVSource(file: file, configuration: CSVConfiguration(), indexer: indexer)
        #expect(source.rowCount > 0)
        #expect(!source.isEditable(row: 0))
        #expect(throws: CSVError.rowNotEditable) { try source.setCell("x", row: 0, column: 0) }
    }

    @Test func partialIndexHidesTheRowStillBeingScanned() throws {
        let dir = TempDirectory()
        let url = dir.url.appendingPathComponent("big.csv")
        try generateLargeCSV(at: url, targetBytes: 300_000, columns: 3, quoted: false,
                             multilineEvery: nil, crlf: false, bom: false, seed: 1)
        let file = try MappedFile(url: url)
        let indexer = LargeFileIndexer(file: file, configuration: CSVConfiguration(),
                                       stepBytes: 4000, progressInterval: 0)
        indexer.start { _ in indexer.cancel() }
        indexer.wait()
        let source = LargeCSVSource(file: file, configuration: CSVConfiguration(), indexer: indexer)
        let last = source.rowCount - 1
        #expect(source.cell(row: last, column: 2).hasSuffix(".2"))
        #expect(source.cell(row: last + 1, column: 0) == "")
    }

    @Test func staleFileReturnsEmptyCells() throws {
        let dir = TempDirectory()
        let url = dir.url.appendingPathComponent("big.csv")
        try generateLargeCSV(at: url, targetBytes: 300_000, columns: 3, quoted: false,
                             multilineEvery: nil, crlf: false, bom: false, seed: 1)
        let source = makeSource(try MappedFile(url: url))
        #expect(!source.cell(row: 2000, column: 0).isEmpty)
        try FileHandle(forWritingTo: url).truncate(atOffset: 100)
        #expect(source.staleness == .replacedOrTruncated)
        #expect(source.cell(row: 3000, column: 0) == "")
        #expect(!source.isEditable(row: 0))
    }

    @Test func blockCacheHitRateOnSequentialRead() throws {
        let dir = TempDirectory()
        let url = dir.url.appendingPathComponent("big.csv")
        try generateLargeCSV(at: url, targetBytes: 300_000, columns: 4, quoted: false,
                             multilineEvery: nil, crlf: false, bom: false, seed: 1)
        let source = makeSource(try MappedFile(url: url))
        for row in 0..<source.rowCount {
            for column in 0..<source.columnCount { _ = source.cell(row: row, column: column) }
        }
        let total = source.cacheHits + source.cacheMisses
        #expect(Double(source.cacheHits) / Double(total) > 0.95)
        #expect(source.cacheMisses <= source.rowCount / LargeFile.blockRows + 2)
    }

    @Test func malformedBlockShowsRawLine() throws {
        let dir = TempDirectory()
        let source = try textSource("a,b\nx\"y\"z,2\nc,d\n", dir)
        #expect(source.cell(row: 1, column: 0) == "x\"y\"z,2")
        #expect(source.cell(row: 1, column: 1) == "")
        #expect(source.cell(row: 2, column: 0) == "c,d")
        #expect(!source.isEditable(row: 1))
    }

    @Test func headerCanBeToggledAfterwards() throws {
        let dir = TempDirectory()
        let source = try textSource("a,b\n1,2\n3,4\n", dir)
        #expect(source.rowCount == 3 && source.headers == nil)
        source.setHeaderEnabled(true)
        #expect(source.rowCount == 2)
        #expect(source.headers == ["a", "b"])
        #expect(source.cell(row: 0, column: 0) == "1")
        source.setHeaderEnabled(false)
        #expect(source.rowCount == 3)
        #expect(source.cell(row: 0, column: 0) == "a")
    }
}
