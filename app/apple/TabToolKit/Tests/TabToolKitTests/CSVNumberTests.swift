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

/// The scanner must agree with the regex on every ASCII input, including the
/// control characters and unusual shapes a fixed table would miss.
@Test func scannerAgreesWithRegexOnRandomAsciiText() {
    var generator = SeededGenerator(seed: 11)
    let alphabet = Array("0123456789.,+-eE \t\n\r\u{0B}\u{0C}x").map(String.init)
    for _ in 0..<50_000 {
        let length = Int(generator.next() % 8)
        let text = (0..<length).map { _ in alphabet[Int(generator.next() % UInt64(alphabet.count))] }.joined()
        for mark in [".", ","] {
            #expect(CSVNumber.scanNumeric(text, decimalMark: mark)
                    == CSVNumber.matchesRegex(text, decimalMark: mark), "\(text.debugDescription) \(mark)")
        }
    }
}

@Test func nonAsciiTextAndMarksFallBackToTheRegex() {
    #expect(CSVNumber.scanNumeric("1\u{00A0}", decimalMark: ".") == nil)
    #expect(CSVNumber.scanNumeric("\u{0663}", decimalMark: ".") == nil)
    #expect(CSVNumber.scanNumeric("1.5", decimalMark: "ab") == nil)
    #expect(CSVNumber.isNumeric("\u{0663}", decimalMark: ".") == CSVNumber.matchesRegex("\u{0663}", decimalMark: "."))
}
