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
        let data = makeBenchmarkCSV(rows: 20_000, cols: 20, quoted: true)
        measure {
            let config = CSVFormatDetector(data: data).detect()
            XCTAssertEqual(config.columnSeparator, ",")
            XCTAssertEqual(config.decimalMark, ".")
            XCTAssertTrue(config.firstRowAsHeader)
        }
    }
}
