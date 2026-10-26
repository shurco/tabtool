import Foundation
import Testing
@testable import TabToolKit

@Suite struct CSVClipboardTests {
    @Test func parseTSVHandlesCRLFAndTrailingNewline() {
        #expect(CSVClipboard.parseTSV("a\tb\r\nc\td\r\n") == [["a", "b"], ["c", "d"]])
    }

    @Test func parseTSVKeepsEmptyFieldsAndBlankRows() {
        #expect(CSVClipboard.parseTSV("a\t\tc\n\n") == [["a", "", "c"], [""]])
    }

    @Test func parseTSVHandlesCRLFMixedWithOtherBreaks() {
        #expect(CSVClipboard.parseTSV("a\tb\rc\td\ne\tf") == [
            ["a", "b"], ["c", "d"], ["e", "f"]
        ])
        #expect(CSVClipboard.parseTSV("a\r\nb\rc\nd") == [["a"], ["b"], ["c"], ["d"]])
    }

    @Test func parseTSVDropsExactlyOneEmptyRowFromTrailingNewline() {
        #expect(CSVClipboard.parseTSV("a\n") == [["a"]])
        #expect(CSVClipboard.parseTSV("a\n\n") == [["a"], [""]])
        #expect(CSVClipboard.parseTSV("a\n\n\n") == [["a"], [""], [""]])
        #expect(CSVClipboard.parseTSV("\n") == [[""]])
    }

    @Test func parseTSVPreservesEmptyFieldsAtEdgesAndMiddle() {
        #expect(CSVClipboard.parseTSV("\ta\t\t") == [["", "a", "", ""]])
        #expect(CSVClipboard.parseTSV("\t\t") == [["", "", ""]])
    }

    @Test func parseTSVEmptyInputReturnsOneEmptyField() {
        #expect(CSVClipboard.parseTSV("") == [[""]])
    }

    @Test func parseTSVDoesNotTrimOrInterpretQuotes() {
        #expect(CSVClipboard.parseTSV(" a \t\"b\tc\"") == [[" a ", "\"b", "c\""]])
    }

    @Test func makeTSVReplacesTabsAndNewlinesInCells() {
        #expect(CSVClipboard.makeTSV(rows: [["a\tb", "c\nd"]],
                                     numericDecimalMark: ".") == "a b\tc d")
        #expect(CSVClipboard.makeTSV(rows: [["a\rb"]],
                                     numericDecimalMark: ".") == "a b")
        #expect(CSVClipboard.makeTSV(rows: [["a\r\nb"]],
                                     numericDecimalMark: ".") == "a b")
    }

    @Test func makeTSVLocalizesNumericCells() {
        let us = Locale(identifier: "en_US")
        #expect(CSVClipboard.makeTSV(rows: [["1,5", "x"]], numericDecimalMark: ",",
                                     locale: us) == "1.5\tx")
    }

    @Test func makeTSVUsesRequestedLocaleForLocalization() {
        let german = Locale(identifier: "de_DE")
        #expect(CSVClipboard.makeTSV(rows: [["1.5", "2"]], numericDecimalMark: ".",
                                     locale: german) == "1,5\t2")
    }

    @Test func makeTSVLeavesNonNumericTextUnchanged() {
        let us = Locale(identifier: "en_US")
        #expect(CSVClipboard.makeTSV(rows: [["a,5", "1,2,3"]], numericDecimalMark: ",",
                                     locale: us) == "a,5\t1,2,3")
    }

    @Test func makeTSVJoinsRowsWithLineFeedsAndNoTrailingNewline() {
        #expect(CSVClipboard.makeTSV(rows: [["a", "b"], ["c", "d"]],
                                     numericDecimalMark: ".") == "a\tb\nc\td")
    }

    @Test func makeTSVPreservesEmptyCellsAndRows() {
        #expect(CSVClipboard.makeTSV(rows: [["", "a"], [""]],
                                     numericDecimalMark: ".") == "\ta\n")
        #expect(CSVClipboard.makeTSV(rows: [], numericDecimalMark: ".") == "")
    }
}
