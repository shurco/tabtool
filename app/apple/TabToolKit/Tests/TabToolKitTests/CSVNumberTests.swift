import Foundation
import Testing
@testable import TabToolKit

@Test(arguments: [
    ("1", ".", true), ("1.5", ".", true), ("-1.5", ".", true),
    ("+1e3", ".", true), (" 42 ", ".", true), ("001", ".", true),
    ("1,5", ".", false), ("1.2.3", ".", false), ("", ".", false),
    ("1,5", ",", true), ("1.5", ",", false), ("1 2", ".", false)
])
func numericClassification(text: String, mark: String, expected: Bool) {
    #expect(CSVNumber.isNumeric(text, decimalMark: mark) == expected)
}

@Test func localizedReplacesOnlyNumericDecimalMark() {
    let us = Locale(identifier: "en_US")
    #expect(CSVNumber.localized("1,50", decimalMark: ",", locale: us) == "1.50")
    #expect(CSVNumber.localized("hello,world", decimalMark: ",", locale: us) == "hello,world")
}

@Test func localizedUsesLocaleDecimalSeparator() {
    let german = Locale(identifier: "de_DE")
    #expect(CSVNumber.localized("1.50", decimalMark: ".", locale: german) == "1,50")
}
