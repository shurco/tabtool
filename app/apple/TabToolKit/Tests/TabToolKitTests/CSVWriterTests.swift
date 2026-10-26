import Foundation
import Testing
@testable import TabToolKit

@Test func quotesEveryFieldAndOmitsTrailingNewline() throws {
    let t = CSVTable(rows: [["a","b"]], headers: ["h1","h2"], minimumColumnCount: 2)
    let s = try CSVWriter(table: t, header: ["h1","h2"], configuration: .init()).string()
    #expect(s == "\"h1\",\"h2\"\n\"a\",\"b\"")
}

@Test func rowsJoinWithLineFeedOnly() throws {
    let t = CSVTable(rows: [["a"],["b"]], headers: nil)
    let s = try CSVWriter(table: t, header: nil, configuration: .init()).string()
    #expect(s == "\"a\"\n\"b\"")
}

@Test func writerUsesVisualColumnOrder() throws {
    var t = CSVTable(rows: [["a","b"]], headers: nil)
    t.moveColumn(from: 1, to: 0)
    let s = try CSVWriter(table: t, header: nil, configuration: .init()).string()
    #expect(s == "\"b\",\"a\"")
}

@Test func doublesQuotesWithQuoteEscape() throws {
    let t = CSVTable(rows: [["a\"b"]], headers: nil)
    let s = try CSVWriter(table: t, header: nil, configuration: .init()).string()
    #expect(s == "\"a\"\"b\"")
}

@Test func usesBackslashEscape() throws {
    let t = CSVTable(rows: [["a\"b"]], headers: nil)
    let c = CSVConfiguration(escapeCharacter: "\\")
    let s = try CSVWriter(table: t, header: nil, configuration: c).string()
    #expect(s == "\"a\\\"b\"")
}

@Test func doublesEscapeCharacter() throws {
    let t = CSVTable(rows: [["a\\b"]], headers: nil)
    let c = CSVConfiguration(escapeCharacter: "\\")
    let s = try CSVWriter(table: t, header: nil, configuration: c).string()
    #expect(s == #""a\\b""#)
    var parser = CSVParser(string: s, configuration: c)
    #expect(try parser.readAll() == [["a\\b"]])
}

@Test func quotedNewlineSurvives() throws {
    let t = CSVTable(rows: [["a\nb"]], headers: nil)
    let s = try CSVWriter(table: t, header: nil, configuration: .init()).string()
    #expect(s == "\"a\nb\"")
}

@Test func noQuoteAmbiguityThrowsUnlessAllowed() throws {
    let t = CSVTable(rows: [["a,b"]], headers: nil)
    let c = CSVConfiguration(quoteCharacter: "")
    #expect(throws: CSVError.ambiguousOutput) {
        _ = try CSVWriter(table: t, header: nil, configuration: c).string()
    }
    let allowed = try CSVWriter(table: t, header: nil, configuration: c,
                                allowAmbiguousOutput: true).string()
    #expect(allowed == "a,b")
}

@Test func noQuoteLineBreakIsAmbiguous() throws {
    let t = CSVTable(rows: [["a\nb"]], headers: nil)
    let c = CSVConfiguration(quoteCharacter: "")
    #expect(CSVWriteValidation.issue(table: t, header: nil,
                                     configuration: c) == .ambiguousOutput)
    #expect(throws: CSVError.ambiguousOutput) {
        _ = try CSVWriter(table: t, header: nil, configuration: c).string()
    }
}

@Test func unencodableTextThrows() throws {
    let t = CSVTable(rows: [["Привет"]], headers: nil)
    let c = CSVConfiguration(encoding: .windowsCP1252)
    #expect(throws: CSVError.unencodableText) {
        _ = try CSVWriter(table: t, header: nil, configuration: c).data()
    }
}

@Test func utf8BOMIsPrepended() throws {
    let t = CSVTable(rows: [["a"]], headers: nil)
    let data = try CSVWriter(table: t, header: nil,
                             configuration: .init(), includeUTF8BOM: true).data()
    #expect(data.starts(with: [0xEF, 0xBB, 0xBF]))
}

@Test func utf8BOMIsOnlyPrependedForUTF8() throws {
    let t = CSVTable(rows: [["a"]], headers: nil)
    let c = CSVConfiguration(encoding: .windowsCP1252, quoteCharacter: "")
    let data = try CSVWriter(table: t, header: nil, configuration: c,
                             includeUTF8BOM: true).data()
    #expect(data == Data("a".utf8))
}

@Test func encodesWithConfiguredEncoding() throws {
    let t = CSVTable(rows: [["café"]], headers: nil)
    let c = CSVConfiguration(encoding: .windowsCP1252, quoteCharacter: "")
    let data = try CSVWriter(table: t, header: nil, configuration: c).data()
    #expect(data == Data([0x63, 0x61, 0x66, 0xE9]))
}

@Test func writerDoesNotConvertDecimalMark() throws {
    let t = CSVTable(rows: [["1,5"]], headers: nil)
    let c = CSVConfiguration(decimalMark: ".")
    let s = try CSVWriter(table: t, header: nil, configuration: c).string()
    #expect(s == "\"1,5\"")
}

@Test func writesHeaderWhenThereAreNoRows() throws {
    let t = CSVTable(rows: [], headers: ["h1","h2"])
    let s = try CSVWriter(table: t, header: t.headers, configuration: .init()).string()
    #expect(s == "\"h1\",\"h2\"")
}

@Test func emptyHeaderlessTableWritesEmptyOutput() throws {
    let t = CSVTable(columns: [])
    let writer = CSVWriter(table: t, header: nil, configuration: .init())
    #expect(try writer.string() == "")
    #expect(try writer.data().isEmpty)
}

@Test func validationDetectsAmbiguityAndEncoding() {
    let noQuote = CSVConfiguration(quoteCharacter: "")
    let ambiguous = CSVTable(rows: [["a,b"]], headers: nil)
    #expect(CSVWriteValidation.issue(table: ambiguous, header: nil,
                                     configuration: noQuote) == .ambiguousOutput)
    let cyrillic = CSVTable(rows: [["Привет"]], headers: nil)
    #expect(CSVWriteValidation.issue(table: cyrillic, header: nil,
                                     configuration: .init(encoding: .windowsCP1252)) == .unencodableText)
    #expect(CSVWriteValidation.issue(table: ambiguous, header: nil,
                                     configuration: .init()) == nil)
}

@Test func validationChecksHeaderAmbiguity() {
    let t = CSVTable(rows: [["x"]], headers: nil)
    let noQuote = CSVConfiguration(quoteCharacter: "")
    #expect(CSVWriteValidation.issue(table: t, header: ["a,b"],
                                     configuration: noQuote) == .ambiguousOutput)
}

@Test func validationChecksHeaderEncoding() {
    let t = CSVTable(rows: [["x"]], headers: nil)
    let cp1252 = CSVConfiguration(encoding: .windowsCP1252)
    #expect(CSVWriteValidation.issue(table: t, header: ["Привет"],
                                     configuration: cp1252) == .unencodableText)
}

@Test func validationMatchesWriterIssueOrder() throws {
    let t = CSVTable(rows: [["Привет, x"]], headers: nil)
    let c = CSVConfiguration(encoding: .windowsCP1252, quoteCharacter: "")
    #expect(CSVWriteValidation.issue(table: t, header: nil,
                                     configuration: c) == .ambiguousOutput)
    #expect(throws: CSVError.ambiguousOutput) {
        _ = try CSVWriter(table: t, header: nil, configuration: c).data()
    }
}
