import Foundation
import Testing
@testable import TabToolKit

private func sniff(_ bytes: [UInt8], prefersChinese: Bool = false) throws -> CSVConfiguration {
    let dir = TempDirectory()
    return try LargeFileSniffer.sniff(MappedFile(url: dir.file("s.csv", bytes)),
                                      prefersChineseEncoding: prefersChinese)
}

private func sniff(_ text: String) throws -> CSVConfiguration {
    try sniff(Array(text.utf8))
}

@Suite struct LargeFileSnifferTests {
    @Test func detectsUTF8Comma() throws {
        let config = try sniff("name,qty\nwidget,3\ngadget,4\n")
        #expect(config.encoding == .utf8)
        #expect(config.columnSeparator == ",")
        #expect(config.firstRowAsHeader)
    }

    @Test func detectsSemicolonWithDecimalComma() throws {
        let config = try sniff("a;b\n1,5;2,5\n3,5;4,5\n")
        #expect(config.columnSeparator == ";")
        #expect(config.decimalMark == ",")
    }

    @Test func detectsTabs() throws {
        let config = try sniff("a\tb\n1\t2\n3\t4\n")
        #expect(config.columnSeparator == "\t")
    }

    @Test func acceptsCP1252() throws {
        let config = try sniff([0x63, 0x61, 0x66, 0xE9, 0x2C, 0x31, 0x0A, 0x74, 0x65, 0xE9, 0x2C, 0x32, 0x0A])
        #expect(config.encoding == .windowsCP1252)
    }

    @Test func emptyFileSniffsToDefaults() throws {
        #expect(try sniff([]).encoding == .utf8)
    }

    @Test(arguments: [
        [UInt8]([0xFF, 0xFE, 0x61, 0x00, 0x2C, 0x00, 0x62, 0x00]),
        [UInt8]([0xFE, 0xFF, 0x00, 0x61, 0x00, 0x2C, 0x00, 0x62]),
        [UInt8]([0x00, 0x00, 0xFE, 0xFF, 0x00, 0x00, 0x00, 0x61]),
        [UInt8]([0x61, 0x00, 0x2C, 0x00, 0x62, 0x00, 0x0A, 0x00]),
        [UInt8]([0x61, 0x2C, 0x1B, 0x24, 0x42, 0x30, 0x6C, 0x1B, 0x28, 0x42, 0x0A])
    ])
    func refusesNonByteScannableFiles(_ bytes: [UInt8]) {
        #expect(throws: CSVError.unsupportedLargeFileEncoding) { try sniff(bytes) }
    }

    @Test func partialMultibyteAtSampleCut() throws {
        let row = Array("é,é\n".utf8)
        var bytes: [UInt8] = []
        while bytes.count < LargeFile.sampleBytes + 100 { bytes += row }
        #expect(LargeFile.sampleBytes % row.count != 0)
        let config = try sniff(bytes)
        #expect(config.encoding == .utf8)
    }

    @Test func sampleCutInsideQuotedMultilineField() throws {
        var bytes = Array(repeating: Array("x,y\n".utf8), count: 262_143).flatMap { $0 }
        bytes += Array("\"ab\ncd\",z\n".utf8)
        bytes += Array(repeating: Array("x,y\n".utf8), count: 100).flatMap { $0 }
        let config = try sniff(bytes)
        #expect(config.columnSeparator == ",")
    }

    @Test func strictParseFailureBeyondTheDetectorRowsSurfaces() {
        let text = "a,b\n1,2\n3,4\n5,6\n7,8\n9,10\nx\"y\"z,1\n"
        #expect(throws: CSVError.quoteInUnquotedField) { try sniff(text) }
    }

    @Test func unbalancedQuoteFallsBackToQuoteless() throws {
        #expect(try sniff("a,b\n\"x,y\n1,2\n").quoteCharacter == "")
    }

    @Test(arguments: [
        CSVConfiguration(columnSeparator: "§"),
        CSVConfiguration(columnSeparator: "||"),
        CSVConfiguration(quoteCharacter: "\u{201C}", escapeCharacter: "\u{201C}"),
        CSVConfiguration(escapeCharacter: "¦")
    ])
    func rejectsMultibyteDelimiters(_ configuration: CSVConfiguration) {
        #expect(throws: CSVError.unsupportedLargeFileDelimiter) {
            try LargeFileSniffer.validate(configuration)
        }
    }

    @Test func acceptsAsciiConfigurations() throws {
        try LargeFileSniffer.validate(CSVConfiguration())
        try LargeFileSniffer.validate(CSVConfiguration(columnSeparator: "|", quoteCharacter: "",
                                                       escapeCharacter: "\\"))
    }

    @Test func validateRejectsRefusedEncodings() {
        #expect(throws: CSVError.unsupportedLargeFileEncoding) {
            try LargeFileSniffer.validate(CSVConfiguration(encoding: .utf16))
        }
    }

    @Test func byteScannableSet() {
        let accepted: [String.Encoding] = [.utf8, .macOSRoman, .windowsCP1252, .isoLatin2,
                                           .windowsCP1250, .windowsCP1251, .windowsCP1253,
                                           .windowsCP1254, SupportedEncodings.gbk]
        for encoding in accepted { #expect(LargeFileSniffer.isByteScannable(encoding)) }
        let refused: [String.Encoding] = [.utf16, .utf16BigEndian, .utf16LittleEndian, .utf32,
                                          .utf32BigEndian, .utf32LittleEndian, .shiftJIS,
                                          .japaneseEUC, .iso2022JP]
        for encoding in refused { #expect(!LargeFileSniffer.isByteScannable(encoding)) }
    }

    @Test func checkAcceptsAnExplicitConfiguration() throws {
        let dir = TempDirectory()
        let file = try MappedFile(url: dir.file("s.csv", Array("a;b\n1;2\n".utf8)))
        try LargeFileSniffer.check(file, configuration: CSVConfiguration(columnSeparator: ";"))
    }

    @Test func checkRefusesUnsupportedEncodingAndMalformedSample() throws {
        let dir = TempDirectory()
        let file = try MappedFile(url: dir.file("s.csv", Array("a,\"b\nc".utf8)))
        #expect(throws: CSVError.unsupportedLargeFileEncoding) {
            try LargeFileSniffer.check(file, configuration: CSVConfiguration(encoding: .utf16))
        }
        #expect(throws: CSVError.malformedQuotedField) {
            try LargeFileSniffer.check(file, configuration: CSVConfiguration())
        }
    }

    @Test func checkRefusesAFileThatLooksLikeUTF16() throws {
        let dir = TempDirectory()
        let file = try MappedFile(url: dir.file("s.csv", [0xFF, 0xFE, 0x61, 0x00, 0x2C, 0x00]))
        #expect(throws: CSVError.unsupportedLargeFileEncoding) {
            try LargeFileSniffer.check(file, configuration: CSVConfiguration())
        }
    }
}
