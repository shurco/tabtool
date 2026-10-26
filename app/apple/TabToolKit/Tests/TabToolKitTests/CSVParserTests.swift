import Foundation
import Testing
@testable import TabToolKit

private func parseAll(_ data: Data, _ configuration: CSVConfiguration) throws -> [[String]] {
    var parser = try CSVParser(data: data, configuration: configuration)
    return try parser.readAll()
}

@Test(arguments: [
    ("comma-separated", CSVConfiguration(quoteCharacter: ""), 3, 3),
    ("semicolon-separated",
     CSVConfiguration(columnSeparator: ";", quoteCharacter: "", decimalMark: ","), 3, 3)
])
func readsUnquotedRows(name: String, config: CSVConfiguration,
                       rowCount: Int, fieldCount: Int) throws {
    let rows = try parseAll(fixture(name), config)
    #expect(rows.count == rowCount)
    #expect(rows.allSatisfy { $0.count == fieldCount })
    for cell in try #require(rows.last) {
        #expect(CSVNumber.isNumeric(cell, decimalMark: config.decimalMark))
    }
}

@Test func quotedRowsUseDecimalComma() throws {
    let config = CSVConfiguration(decimalMark: ",")
    let rows = try parseAll(fixture("comma-separated-quote"), config)
    #expect(rows.count == 3)
    #expect(rows.allSatisfy { $0.count == 3 })
    for row in rows.dropFirst() {
        for cell in row { #expect(CSVNumber.isNumeric(cell, decimalMark: ",")) }
    }
}

@Test func quotedFieldPreservesSeparatorAndNewline() throws {
    let rows = try parseAll(Data("a,\"b,c\nd\",e".utf8), .init())
    #expect(rows == [["a", "b,c\nd", "e"]])
}

@Test func blankLinesBecomeEmptyRows() throws {
    let rows = try parseAll(fixture("blank-lines"), .init())
    #expect(rows.count == 6)
    #expect(rows[2] == [""])
    #expect(rows[3] == [""])
    #expect(rows.first == ["1"])
    #expect(rows.last == ["6"])
}

@Test func emptyInputReturnsOneEmptyRow() throws {
    #expect(try parseAll(Data(), .init()) == [[""]])
}

@Test func trailingEmptyFieldsArePreserved() throws {
    #expect(try parseAll(Data("a,b,\n".utf8), .init()) == [["a", "b", ""]])
    #expect(try parseAll(Data("a,b,,\n".utf8), .init()) == [["a", "b", "", ""]])
}

@Test func crlfAndCrAreSingleRowBreaks() throws {
    let rows = try parseAll(Data("a,b\r\nc,d\re,f\n".utf8), .init())
    #expect(rows == [["a", "b"], ["c", "d"], ["e", "f"]])
}

@Test func unicodeLineSeparatorsBreakRows() throws {
    let rows = try parseAll(Data("a,b\u{0085}c,d\u{2028}e,f\u{2029}g,h".utf8), .init())
    #expect(rows == [["a", "b"], ["c", "d"], ["e", "f"], ["g", "h"]])
}

@Test func stringInitializerAndReset() throws {
    var parser = CSVParser(string: "a;b\nc;d",
                           configuration: .init(columnSeparator: ";"))
    #expect(!parser.hadUTF8BOM)
    #expect(try parser.nextRow() == ["a", "b"])
    #expect(!parser.isAtEnd)
    #expect(try parser.nextRow() == ["c", "d"])
    #expect(parser.isAtEnd)
    #expect(try parser.nextRow() == nil)

    parser.reset()
    #expect(!parser.isAtEnd)
    #expect(try parser.readAll() == [["a", "b"], ["c", "d"]])
}

@Test(arguments: [
    ("missing-quote-atEnd", CSVConfiguration()),
    ("tooMany-quotes-atEnd", CSVConfiguration()),
    ("missing-backslash-beforeValue", CSVConfiguration(escapeCharacter: "\\")),
    ("missing-backslash-afterValue", CSVConfiguration(escapeCharacter: "\\")),
    ("missing-value-for-backslash-inquote", CSVConfiguration(escapeCharacter: "\\")),
    ("missing-value-for-backslash", CSVConfiguration(escapeCharacter: "\\"))
])
func malformedQuotedFixturesThrow(name: String, config: CSVConfiguration) throws {
    let data = try fixture(name)
    #expect(throws: CSVError.malformedQuotedField) {
        _ = try parseAll(data, config)
    }
}

@Test func quoteInUnquotedValueThrows() throws {
    let data = try fixture("quote-in-unquoted-value")
    #expect(throws: CSVError.quoteInUnquotedField) {
        _ = try parseAll(data, .init())
    }
}

@Test func contentAfterClosingQuoteThrows() {
    #expect(throws: CSVError.malformedQuotedField) {
        _ = try parseAll(Data("\"a\"b,c".utf8), .init())
    }
}

@Test func invalidUTF8ThrowsCannotDecode() {
    #expect(throws: CSVError.cannotDecodeData) {
        _ = try CSVParser(data: Data([0xFF, 0xFE, 0xFD]), configuration: .init())
    }
}

@Test func doubledQuoteEscapeDecodesExactCells() throws {
    let data = try fixture("quote-quote-escape")
    let rows = try parseAll(data, .init())
    #expect(rows == [["test\" test", "test"], ["1.2", "1,3"]])
    try #require(rows.count == 2)
    try #require(rows[1].count == 2)
    #expect(CSVNumber.isNumeric(rows[1][0], decimalMark: "."))
    #expect(!CSVNumber.isNumeric(rows[1][1], decimalMark: "."))
}

@Test func backslashEscapeDecodesExactCells() throws {
    let data = try fixture("quote-backslash-escape")
    let rows = try parseAll(data, CSVConfiguration(escapeCharacter: "\\"))
    #expect(rows == [["test\" test", "test"], ["1.2", "1,3"]])
}

@Test func noQuoteModeTreatsQuotesAsText() throws {
    let rows = try parseAll(Data("a\"b,c".utf8), CSVConfiguration(quoteCharacter: ""))
    #expect(rows == [["a\"b", "c"]])
}

@Test func utf8BOMIsStrippedAndFlagged() throws {
    var data = Data([0xEF, 0xBB, 0xBF])
    data.append(Data("a,b".utf8))
    let parser = try CSVParser(data: data, configuration: .init())
    #expect(parser.hadUTF8BOM)
    var mutable = parser
    #expect(try mutable.readAll() == [["a", "b"]])
}
