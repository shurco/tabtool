import Foundation
import Testing
@testable import TabToolKit

@Test(arguments: [
    ("comma-separated", CSVConfiguration(quoteCharacter: "")),
    ("semicolon-separated", CSVConfiguration(columnSeparator: ";", quoteCharacter: "", decimalMark: ",")),
    ("comma-separated-quote", CSVConfiguration(decimalMark: ",")),
    ("quote-quote-escape", CSVConfiguration()),
    ("quote-backslash-escape", CSVConfiguration(escapeCharacter: "\\")),
    ("blank-lines", CSVConfiguration(quoteCharacter: ""))
])
func roundTripPreservesLogicalTable(name: String, config: CSVConfiguration) throws {
    var parser = try CSVParser(data: try fixture(name), configuration: config)
    let rows = try parser.readAll()
    let header = config.firstRowAsHeader ? rows.first : nil
    let body = config.firstRowAsHeader ? Array(rows.dropFirst()) : rows
    let table = CSVTable(rows: body, headers: header)
    let data = try CSVWriter(table: table, header: header, configuration: config).data()
    var reparser = try CSVParser(data: data, configuration: config)
    let reparsed = try reparser.readAll()
    let newHeader = config.firstRowAsHeader ? reparsed.first : nil
    let newBody = config.firstRowAsHeader ? Array(reparsed.dropFirst()) : reparsed
    let newTable = CSVTable(rows: newBody, headers: newHeader)
    #expect(newTable.headers == table.headers)
    #expect(newTable.columns.map(\.cells) == table.columns.map(\.cells))
}

@Test func randomTablesRoundTrip() throws {
    var rng = SeededGenerator(seed: 42)
    for _ in 0..<100 {
        let rows = (0..<Int.random(in: 1...8, using: &rng)).map { _ in
            (0..<Int.random(in: 1...5, using: &rng)).map { _ in
                ["a", "1,5", "x\"y", "line\nbreak", ""].randomElement(using: &rng)!
            }
        }
        let table = CSVTable(rows: rows, headers: nil)
        let config = CSVConfiguration(decimalMark: ",")
        let data = try CSVWriter(table: table, header: nil, configuration: config).data()
        var parser = try CSVParser(data: data, configuration: config)
        let parsed = try parser.readAll()
        #expect(CSVTable(rows: parsed, headers: nil).columns.map(\.cells)
                == table.columns.map(\.cells))
    }
}

@Test func randomTablesWithBackslashEscapeRoundTrip() throws {
    var rng = SeededGenerator(seed: 7)
    let values = ["a", "1,5", "x\\y", "x\"y", "x\\\"y", "line\nbreak",
                  "", "a,b", "a;b"]
    for _ in 0..<100 {
        let rows = (0..<Int.random(in: 1...8, using: &rng)).map { _ in
            (0..<Int.random(in: 1...5, using: &rng)).map { _ in
                values.randomElement(using: &rng)!
            }
        }
        let table = CSVTable(rows: rows, headers: nil)
        let config = CSVConfiguration(columnSeparator: ";",
                                      escapeCharacter: "\\",
                                      decimalMark: ",")
        let data = try CSVWriter(table: table, header: nil, configuration: config).data()
        var parser = try CSVParser(data: data, configuration: config)
        let parsed = try parser.readAll()
        #expect(CSVTable(rows: parsed, headers: nil).columns.map(\.cells)
                == table.columns.map(\.cells))
    }
}

@Test func randomTablesWithHeadersRoundTrip() throws {
    var rng = SeededGenerator(seed: 99)
    let values = ["h", "a,b", "x\"y", "", "line\nbreak"]
    for _ in 0..<100 {
        let rows = (0..<Int.random(in: 1...8, using: &rng)).map { _ in
            (0..<Int.random(in: 1...5, using: &rng)).map { _ in
                values.randomElement(using: &rng)!
            }
        }
        var table = CSVTable(rows: rows, headers: nil)
        table.setHeaders((0..<table.columns.count).map { _ in
            values.randomElement(using: &rng)!
        })
        let config = CSVConfiguration(decimalMark: ",")
        let data = try CSVWriter(table: table, header: table.headers,
                                 configuration: config).data()
        var parser = try CSVParser(data: data, configuration: config)
        let parsed = try parser.readAll()
        let parsedTable = CSVTable(rows: Array(parsed.dropFirst()), headers: parsed.first)
        #expect(parsedTable == table)
    }
}
