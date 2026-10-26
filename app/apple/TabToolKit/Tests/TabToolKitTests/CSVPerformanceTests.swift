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
}
