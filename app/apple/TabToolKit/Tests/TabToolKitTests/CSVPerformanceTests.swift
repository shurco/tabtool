import Foundation
import XCTest
@testable import TabToolKit

final class CSVPerformanceTests: XCTestCase {
    func testReadsLargeUnquotedFixture() throws {
        let data = try fixtureData("test-doc-unquoted", subdirectory: "Fixtures/Performance")
        let config = CSVConfiguration(quoteCharacter: "")
        measure {
            var parser = try! CSVParser(data: data, configuration: config)
            let rows = try! parser.readAll()
            XCTAssertEqual(rows.count, 987)
            XCTAssertTrue(rows.allSatisfy { $0.count == 12 })
        }
    }

    func testReadsLargeQuotedFixture() throws {
        let data = try fixtureData("test-doc-quoted", subdirectory: "Fixtures/Performance")
        measure {
            var parser = try! CSVParser(data: data, configuration: .init())
            let rows = try! parser.readAll()
            XCTAssertEqual(rows.count, 1002)
            XCTAssertTrue(rows.allSatisfy { $0.count == 11 })
        }
    }

    func testHeuristicOnLargeFixture() throws {
        let data = try fixtureData("test-doc-quoted", subdirectory: "Fixtures/Performance")
        measure {
            let config = CSVFormatDetector(data: data).detect()
            XCTAssertEqual(config.columnSeparator, ",")
            XCTAssertEqual(config.decimalMark, ",")
            XCTAssertEqual(config.quoteCharacter, "\"")
            XCTAssertEqual(config.escapeCharacter, "\"")
            XCTAssertTrue(config.firstRowAsHeader)
        }
    }

    /// Sorting 100K rows by a numeric column. No latency assertion; the
    /// duration is printed for manual tracking.
    func testSortOn100KRows() {
        let rowCount = 100_000
        let table = CSVTable(columns: [
            CSVColumn(header: nil,
                      cells: (0..<rowCount).map { "\(($0 * 7919) % 100_000)" })
        ])

        let start = Date()
        let order = table.rowOrder(sortedByColumn: 0, direction: .ascending,
                                   decimalMark: ".")
        let duration = Date().timeIntervalSince(start)
        print("sort 100K rows: \(duration)s")

        XCTAssertEqual(table.cell(row: order[0], column: 0), "0")
        XCTAssertEqual(table.cell(row: order[rowCount - 1], column: 0), "99999")
    }

    /// Correctness and rough timing for delete/move/save on 100k rows.
    ///
    /// There is no hard latency assertion: CI machines vary. The test records
    /// the three durations and fails only if the resulting table is wrong.
    func testDeleteMoveAndSaveOn100KRows() throws {
        let rowCount = 100_000
        let columns = (0..<2).map { column in
            CSVColumn(header: nil,
                      cells: (0..<rowCount).map { row in "\(column)-\(row)" })
        }
        var table = CSVTable(columns: columns)

        let deleted = IndexSet((0..<rowCount).filter { $0 % 7 == 0 })
        let deleteStart = Date()
        table.deleteRows(at: deleted)
        let deleteDuration = Date().timeIntervalSince(deleteStart)

        let keptCount = rowCount - deleted.count
        XCTAssertEqual(table.rowCount, keptCount)
        XCTAssertEqual(table.cell(row: 0, column: 0), "0-1")
        XCTAssertEqual(table.cell(row: 0, column: 1), "1-1")
        XCTAssertEqual(table.cell(row: keptCount - 1, column: 0),
                       "0-\(rowCount - 1)")

        let moved = IndexSet(stride(from: 0, to: keptCount, by: 3))
        let expectedMoved = moved.map { table.cell(row: $0, column: 0) }
        let moveStart = Date()
        table.moveRows(at: moved, to: keptCount)
        let moveDuration = Date().timeIntervalSince(moveStart)

        XCTAssertEqual(table.rowCount, keptCount)
        XCTAssertEqual(Array(table.columns[0].cells.suffix(moved.count)),
                       expectedMoved)

        let saveStart = Date()
        let data = try CSVWriter(table: table, header: nil,
                                 configuration: .init()).data()
        let saveDuration = Date().timeIntervalSince(saveStart)
        XCTAssertGreaterThan(data.count, 0)

        print(String(format: "100k delete %.3fs, move %.3fs, save %.3fs",
                     deleteDuration, moveDuration, saveDuration))
    }

    /// Detection must decode a large file once per encoding, not once per
    /// candidate configuration.
    func testDetectsFormatOfLargeQuotedFile() {
        let data = makeBenchmarkCSV(rows: 5_000, cols: 20, quoted: true)
        measure {
            let config = CSVFormatDetector(data: data).detect()
            XCTAssertEqual(config.columnSeparator, ",")
            XCTAssertEqual(config.decimalMark, ".")
            XCTAssertTrue(config.firstRowAsHeader)
        }
    }

    /// Writing must not run quote replacements on fields that need none, and
    /// the output must parse back to the same rows.
    func testWritesLargeQuotedTable() throws {
        let data = makeBenchmarkCSV(rows: 5_000, cols: 20, quoted: true)
        var parser = try CSVParser(data: data, configuration: .init())
        let rows = try parser.readAll()
        let table = CSVTable(rows: Array(rows.dropFirst()), headers: rows.first)
        measure {
            let written = try! CSVWriter(table: table, header: table.headers,
                                         configuration: .init()).data()
            var reparsed = try! CSVParser(data: written, configuration: .init())
            XCTAssertEqual(try! reparsed.readAll(), rows)
        }
    }

    /// Sorting text compares with `localizedStandardCompare`; each pair must
    /// be compared once. Empties stay last and ties keep their row order.
    func testSortsTextColumn() {
        let rowCount = 20_000
        var generator = SeededGenerator(seed: 7)
        let cells = (0..<rowCount).map { row in
            row % 50 == 0 ? "" : "text\(generator.next() % 5000)"
        }
        let table = CSVTable(columns: [CSVColumn(header: nil, cells: cells)])
        for direction in [CSVSortDirection.ascending, .descending] {
            var order: [Int] = []
            if direction == .ascending {
                measure {
                    order = table.rowOrder(sortedByColumn: 0, direction: direction,
                                           decimalMark: ".")
                }
            } else {
                order = table.rowOrder(sortedByColumn: 0, direction: direction,
                                       decimalMark: ".")
            }
            let sorted = order.map { cells[$0] }
            XCTAssertEqual(sorted.suffix(rowCount / 50), Array(repeating: "", count: rowCount / 50))
            let text = sorted.filter { !$0.isEmpty }
            XCTAssertEqual(text, text.sorted {
                direction == .ascending ? $0.localizedStandardCompare($1) == .orderedAscending
                    : $0.localizedStandardCompare($1) == .orderedDescending
            })
            let equalRuns = zip(order, order.dropFirst()).filter { cells[$0] == cells[$1] }
            XCTAssertTrue(equalRuns.allSatisfy { $0 < $1 })
        }
    }
}
