import Foundation
import Testing
@testable import TabToolKit

/// The format expected for a heuristic fixture.
struct ExpectedFormat: Sendable {
    let separator: String
    let decimal: String
    let quote: String
    let escape: String
    let header: Bool
}

@Test(arguments: [
    ("heuristic-config1", ExpectedFormat(separator: ",", decimal: ".", quote: "\"", escape: "\"", header: true)),
    ("heuristic-config2", ExpectedFormat(separator: ";", decimal: ",", quote: "\"", escape: "\"", header: true)),
    ("heuristic-config3", ExpectedFormat(separator: "\t", decimal: ".", quote: "\"", escape: "\"", header: true)),
    ("heuristic-config4", ExpectedFormat(separator: "\t", decimal: ",", quote: "\"", escape: "\"", header: true)),
    ("heuristic-config5", ExpectedFormat(separator: ";", decimal: ",", quote: "\"", escape: "\\", header: true)),
    ("heuristic-config6", ExpectedFormat(separator: ",", decimal: ".", quote: "", escape: "\"", header: true)),
    ("heuristic-config7", ExpectedFormat(separator: ";", decimal: ",", quote: "", escape: "\"", header: true)),
    ("heuristic-config8", ExpectedFormat(separator: "\t", decimal: ".", quote: "", escape: "\"", header: true)),
    ("heuristic-config9", ExpectedFormat(separator: "\t", decimal: ",", quote: "", escape: "\"", header: true)),
    ("heuristic-config10", ExpectedFormat(separator: ",", decimal: ".", quote: "\"", escape: "\\", header: true)),
    ("heuristic-config11", ExpectedFormat(separator: ",", decimal: ",", quote: "\"", escape: "\"", header: true))
])
func detectsExpectedFormats(name: String, expected: ExpectedFormat) throws {
    let config = CSVFormatDetector(data: try fixture(name)).detect()
    #expect(config.columnSeparator == expected.separator)
    #expect(config.decimalMark == expected.decimal)
    #expect(config.quoteCharacter == expected.quote)
    #expect(config.escapeCharacter == expected.escape)
    #expect(config.firstRowAsHeader == expected.header)
}

@Test func firstRowNumberMeansNoHeader() throws {
    let config = CSVFormatDetector(data: try fixture("heuristic-first-row-with-number")).detect()
    #expect(config.firstRowAsHeader == false)
}

@Test func shorterFirstRowMeansNoHeader() throws {
    let config = CSVFormatDetector(data: try fixture("heuristic-first-row-one-row-shorter")).detect()
    #expect(config.firstRowAsHeader == false)
}

@Test func longerFirstRowMeansNoHeader() throws {
    let config = CSVFormatDetector(data: try fixture("heuristic-first-row-longer")).detect()
    #expect(config.firstRowAsHeader == false)
}

@Test func cp1252IsDetected() throws {
    let config = CSVFormatDetector(data: try fixture("heuristic-no-utf-encoding")).detect()
    #expect(config.encoding == .windowsCP1252)
}

@Test func gbkIsPreferredForChinese() throws {
    let config = CSVFormatDetector(data: try fixture("heuristic-gbk"),
                                   prefersChineseEncoding: true).detect()
    let gbk = SupportedEncodings.all.first { $0.name == "Chinese (GBK)" }!.encoding
    #expect(config.encoding == gbk)
}

@Test func macRomanTabSampleIsDetected() throws {
    let config = CSVFormatDetector(data: try fixture("macroman-tab-sample")).detect()
    #expect(config.encoding == .macOSRoman)
    #expect(config.columnSeparator == "\t")
    #expect(config.firstRowAsHeader)
}
