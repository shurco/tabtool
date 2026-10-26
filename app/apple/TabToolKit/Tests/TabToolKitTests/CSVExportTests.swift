import Foundation
import Testing
@testable import TabToolKit

@Test func exportConvertsDecimalMarkAndPreservesFormatting() throws {
    let t = CSVTable(rows: [["001,50"]], headers: nil)
    let doc = CSVConfiguration(columnSeparator: ";", decimalMark: ",")
    let out = CSVConfiguration(decimalMark: ".")
    let data = try CSVExport.data(table: t, documentConfiguration: doc,
                                  exportConfiguration: out)
    #expect(String(data: data, encoding: .utf8) == "\"001.50\"")
}

@Test func exportHeaderFallsBackToGeneratedNames() throws {
    let t = CSVTable(rows: [["x","y"]], headers: nil)
    var out = CSVConfiguration()
    out.firstRowAsHeader = true
    let data = try CSVExport.data(table: t, documentConfiguration: .init(),
                                  exportConfiguration: out)
    #expect(String(data: data, encoding: .utf8) == "\"A\",\"B\"\n\"x\",\"y\"")
}

@Test func exportHeaderToggleOffOmitsHeader() throws {
    let t = CSVTable(rows: [["x","y"]], headers: ["h1","h2"])
    var out = CSVConfiguration()
    out.firstRowAsHeader = false
    let data = try CSVExport.data(table: t, documentConfiguration: .init(),
                                  exportConfiguration: out)
    #expect(String(data: data, encoding: .utf8) == "\"x\",\"y\"")
}

@Test func exportUsesDocumentHeaders() throws {
    let t = CSVTable(rows: [["x","y"]], headers: ["h1","h2"])
    var out = CSVConfiguration()
    out.firstRowAsHeader = true
    let data = try CSVExport.data(table: t, documentConfiguration: .init(),
                                  exportConfiguration: out)
    #expect(String(data: data, encoding: .utf8) == "\"h1\",\"h2\"\n\"x\",\"y\"")
}

@Test func exportConvertsACopyOnly() throws {
    let t = CSVTable(rows: [["1,5","text,5"]], headers: nil)
    let doc = CSVConfiguration(decimalMark: ",")
    let out = CSVConfiguration(decimalMark: ".")
    let data = try CSVExport.data(table: t, documentConfiguration: doc,
                                  exportConfiguration: out)
    #expect(String(data: data, encoding: .utf8) == "\"1.5\",\"text,5\"")
    #expect(t.cell(row: 0, column: 0) == "1,5")
    #expect(t.cell(row: 0, column: 1) == "text,5")
}

@Test func exportPropagatesAmbiguityFlag() throws {
    let t = CSVTable(rows: [["a,b"]], headers: nil)
    let noQuote = CSVConfiguration(quoteCharacter: "")
    #expect(throws: CSVError.ambiguousOutput) {
        _ = try CSVExport.data(table: t, documentConfiguration: .init(),
                               exportConfiguration: noQuote)
    }
    let data = try CSVExport.data(table: t, documentConfiguration: .init(),
                                  exportConfiguration: noQuote,
                                  allowAmbiguousOutput: true)
    #expect(String(data: data, encoding: .utf8) == "a,b")
}

@Test func exportRejectsTrailingLineFeed() throws {
    let t = CSVTable(rows: [["a"], [""]], headers: nil)
    let noQuote = CSVConfiguration(quoteCharacter: "")
    #expect(throws: CSVError.ambiguousOutput) {
        _ = try CSVExport.data(table: t, documentConfiguration: .init(),
                               exportConfiguration: noQuote)
    }
    let data = try CSVExport.data(table: t, documentConfiguration: .init(),
                                  exportConfiguration: noQuote,
                                  allowAmbiguousOutput: true)
    #expect(String(data: data, encoding: .utf8) == "a\n")
}

@Test func exportUsesExportConfigurationForQuoting() throws {
    let t = CSVTable(rows: [["1.5"]], headers: nil)
    let doc = CSVConfiguration()
    let commaOut = CSVConfiguration(quoteCharacter: "", decimalMark: ",")
    #expect(throws: CSVError.ambiguousOutput) {
        _ = try CSVExport.data(table: t, documentConfiguration: doc,
                               exportConfiguration: commaOut)
    }

    let semicolonOut = CSVConfiguration(columnSeparator: ";", quoteCharacter: "",
                                        decimalMark: ",")
    let data = try CSVExport.data(table: t, documentConfiguration: doc,
                                  exportConfiguration: semicolonOut)
    #expect(String(data: data, encoding: .utf8) == "1,5")
}

@Test func exportPreflightUsesConvertedDecimalMark() {
    let table = CSVTable(rows: [["1.5"]], headers: nil)
    let document = CSVConfiguration(decimalMark: ".")
    let export = CSVConfiguration(quoteCharacter: "", escapeCharacter: "",
                                  decimalMark: ",")
    #expect(CSVExport.preflightIssue(table: table,
                                     documentConfiguration: document,
                                     exportConfiguration: export)
        == .ambiguousOutput)
}

@Test func exportPreflightIgnoresAmbiguityRemovedByConversion() throws {
    let table = CSVTable(rows: [["1,5"]], headers: nil)
    let document = CSVConfiguration(quoteCharacter: "", escapeCharacter: "",
                                    decimalMark: ",")
    let export = CSVConfiguration(quoteCharacter: "", escapeCharacter: "",
                                  decimalMark: ".")
    #expect(CSVExport.preflightIssue(table: table,
                                     documentConfiguration: document,
                                     exportConfiguration: export) == nil)

    let data = try CSVExport.data(table: table, documentConfiguration: document,
                                  exportConfiguration: export)
    #expect(String(data: data, encoding: .utf8) == "1.5")
}

@Test func exportPreflightAllowsForcedAmbiguousConvertedOutput() throws {
    let table = CSVTable(rows: [["1.5"]], headers: nil)
    let document = CSVConfiguration(decimalMark: ".")
    let export = CSVConfiguration(quoteCharacter: "", escapeCharacter: "",
                                  decimalMark: ",")
    #expect(CSVExport.preflightIssue(table: table,
                                     documentConfiguration: document,
                                     exportConfiguration: export)
        == .ambiguousOutput)

    let data = try CSVExport.data(table: table, documentConfiguration: document,
                                  exportConfiguration: export,
                                  allowAmbiguousOutput: true)
    #expect(String(data: data, encoding: .utf8) == "1,5")
}

@Test func exportPreflightReportsUnencodableText() {
    let table = CSVTable(rows: [["ж"]], headers: nil)
    let export = CSVConfiguration(encoding: .ascii)
    #expect(CSVExport.preflightIssue(table: table,
                                     documentConfiguration: .init(),
                                     exportConfiguration: export)
        == .unencodableText)
}

@Test func exportPreflightChecksTheWrittenHeader() {
    let table = CSVTable(rows: [["x"]], headers: ["a,b"])
    var export = CSVConfiguration(quoteCharacter: "", escapeCharacter: "")
    export.firstRowAsHeader = true
    #expect(CSVExport.preflightIssue(table: table,
                                     documentConfiguration: .init(),
                                     exportConfiguration: export)
        == .ambiguousOutput)
}

@Test func exportPreflightIgnoresAnUnwrittenHeader() {
    let table = CSVTable(rows: [["x"]], headers: ["a,b"])
    let export = CSVConfiguration(quoteCharacter: "", escapeCharacter: "")
    #expect(CSVExport.preflightIssue(table: table,
                                     documentConfiguration: .init(),
                                     exportConfiguration: export) == nil)
}
