import Foundation
import Testing
@testable import TabToolKit

@Suite struct ExportFieldNamesTests {
    @Test func usesDocumentHeaders() {
        let table = CSVTable(rows: [["1", "2"]], headers: ["Name", "Price"])
        #expect(ExportFieldNames.names(for: table) == ["Name", "Price"])
    }

    @Test func fallsBackToColumnLettersWithoutHeaders() {
        let table = CSVTable(rows: [["1", "2", "3"]], headers: nil)
        #expect(ExportFieldNames.names(for: table) == ["A", "B", "C"])
    }

    @Test func emptyHeaderGetsColumnLetter() {
        let table = CSVTable(rows: [["1", "2"]], headers: ["Name", ""])
        #expect(ExportFieldNames.names(for: table) == ["Name", "B"])
    }

    @Test func duplicatesGetNumericSuffix() {
        #expect(ExportFieldNames.unique(["a", "a", "a"]) == ["a", "a_2", "a_3"])
    }

    @Test func suffixSkipsNamesAlreadyTaken() {
        #expect(ExportFieldNames.unique(["a", "a", "a_2"]) == ["a", "a_3", "a_2"])
    }
}

@Suite struct JSONTableWriterTests {
    @Test func writesArrayOfObjectsInColumnOrder() {
        let table = CSVTable(rows: [["Ann", "1.5"], ["Bob", "2"]],
                             headers: ["Name", "Price"])
        #expect(JSONTableWriter.string(table: table) == """
            [
              {"Name": "Ann", "Price": "1.5"},
              {"Name": "Bob", "Price": "2"}
            ]

            """)
    }

    @Test func emptyTableIsEmptyArray() {
        let table = CSVTable(columns: [CSVColumn(header: "A", cells: [])])
        #expect(JSONTableWriter.string(table: table) == "[]\n")
    }

    @Test func escapesQuotesBackslashesAndControlCharacters() {
        #expect(JSONTableWriter.quoted("a\"b\\c/d") == #""a\"b\\c/d""#)
        #expect(JSONTableWriter.quoted("1\n2\r3\t4") == #""1\n2\r3\t4""#)
        #expect(JSONTableWriter.quoted("\u{0}\u{1F}") == #""\u0000\u001F""#)
        #expect(JSONTableWriter.quoted("ж😀") == "\"ж😀\"")
    }

    @Test func outputParsesBackToTheSameValues() throws {
        let table = CSVTable(rows: [["a\"\n\\", "007"]], headers: ["k", "k"])
        let data = Data(JSONTableWriter.string(table: table).utf8)
        let parsed = try #require(
            try JSONSerialization.jsonObject(with: data) as? [[String: String]])
        #expect(parsed == [["k": "a\"\n\\", "k_2": "007"]])
    }
}

@Suite struct XMLTableWriterTests {
    @Test func writesRowElementsNamedByHeaders() {
        let table = CSVTable(rows: [["Ann", "1.5"]], headers: ["Name", "Price"])
        #expect(XMLTableWriter.string(table: table) == """
            <?xml version="1.0" encoding="UTF-8"?>
            <rows>
              <row>
                <Name>Ann</Name>
                <Price>1.5</Price>
              </row>
            </rows>

            """)
    }

    @Test func emptyTableHasOnlyTheRoot() {
        let table = CSVTable(columns: [CSVColumn(header: "A", cells: [])])
        #expect(XMLTableWriter.string(table: table) == """
            <?xml version="1.0" encoding="UTF-8"?>
            <rows>
            </rows>

            """)
    }

    @Test func sanitizesElementNames() {
        #expect(XMLTableWriter.elementName("Unit Price") == "Unit_Price")
        #expect(XMLTableWriter.elementName("2024") == "_2024")
        #expect(XMLTableWriter.elementName("-x") == "_-x")
        #expect(XMLTableWriter.elementName("xmlData") == "_xmlData")
        #expect(XMLTableWriter.elementName("a:b<c>") == "a_b_c_")
        #expect(XMLTableWriter.elementName("Цена.v-1") == "Цена.v-1")
        #expect(XMLTableWriter.elementName("m²") == "m_")
    }

    @Test func sanitizedDuplicatesStayUnique() {
        let table = CSVTable(rows: [["1", "2"]], headers: ["a b", "a_b"])
        let xml = XMLTableWriter.string(table: table)
        #expect(xml.contains("<a_b>1</a_b>"))
        #expect(xml.contains("<a_b_2>2</a_b_2>"))
    }

    @Test func escapesTextAndPreservesCarriageReturns() {
        #expect(XMLTableWriter.escaped("a<b>&c\"'") == "a&lt;b&gt;&amp;c\"'")
        #expect(XMLTableWriter.escaped("1\r\n2\t") == "1&#13;\n2\t")
    }

    /// XML 1.0 cannot represent these characters at all, not even as
    /// character references.
    @Test func replacesCharactersXMLCannotRepresent() {
        #expect(XMLTableWriter.escaped("a\u{0}b\u{1B}c\u{FFFE}") == "a\u{FFFD}b\u{FFFD}c\u{FFFD}")
    }

    @Test func outputIsWellFormed() throws {
        let table = CSVTable(rows: [["<&>\r\n", "\u{1}"]], headers: ["1 a", "xml"])
        let parser = XMLParser(data: Data(XMLTableWriter.string(table: table).utf8))
        #expect(parser.parse())
    }
}

@Suite struct CSVExportFormatTests {
    @Test func structuredFormatsConvertDecimalMark() throws {
        let table = CSVTable(rows: [["1,5"]], headers: ["Price"])
        let document = CSVConfiguration(columnSeparator: ";", decimalMark: ",")
        let export = CSVConfiguration(decimalMark: ".")
        let json = try CSVExport.data(table: table, format: .json,
                                      documentConfiguration: document,
                                      exportConfiguration: export)
        let xml = try CSVExport.data(table: table, format: .xml,
                                     documentConfiguration: document,
                                     exportConfiguration: export)
        #expect(String(decoding: json, as: UTF8.self).contains(#""Price": "1.5""#))
        #expect(String(decoding: xml, as: UTF8.self).contains("<Price>1.5</Price>"))
    }

    /// JSON and XML are always UTF-8, so the CSV encoding and quoting rules
    /// never block them.
    @Test func structuredFormatsIgnoreCSVWriteRules() throws {
        let table = CSVTable(rows: [["ж,\n"]], headers: nil)
        let export = CSVConfiguration(encoding: .ascii, quoteCharacter: "")
        for format in [ExportFormat.json, .xml] {
            #expect(CSVExport.preflightIssue(table: table, format: format,
                                             documentConfiguration: .init(),
                                             exportConfiguration: export) == nil)
            let data = try CSVExport.data(table: table, format: format,
                                          documentConfiguration: .init(),
                                          exportConfiguration: export)
            #expect(String(decoding: data, as: UTF8.self).contains("ж,"))
        }
    }
}
