import Foundation
import Testing
@testable import TabToolKit

@Test func defaultConfigurationMatchesDefaults() {
    let c = CSVConfiguration()
    #expect(c.encoding == .utf8)
    #expect(c.columnSeparator == ",")
    #expect(c.quoteCharacter == "\"")
    #expect(c.escapeCharacter == "\"")
    #expect(c.decimalMark == ".")
    #expect(c.firstRowAsHeader == false)
}

@Test func configurationStoresCustomValues() {
    var c = CSVConfiguration(
        encoding: .windowsCP1252,
        columnSeparator: ";",
        quoteCharacter: "'",
        escapeCharacter: "\\",
        decimalMark: ",",
        firstRowAsHeader: true
    )

    #expect(c.encoding == .windowsCP1252)
    #expect(c.columnSeparator == ";")
    #expect(c.quoteCharacter == "'")
    #expect(c.escapeCharacter == "\\")
    #expect(c.decimalMark == ",")
    #expect(c.firstRowAsHeader)

    c.columnSeparator = "|"
    #expect(c.columnSeparator == "|")
}
