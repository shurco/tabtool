import Testing
@testable import TabToolKit

@Suite struct DecimalMarkConverterTests {
    @Test func convertOnlyNumericCells() {
        #expect(DecimalMarkConverter.convert("1,50", from: ",", to: ".") == "1.50")
        #expect(DecimalMarkConverter.convert("a,b", from: ",", to: ".") == "a,b")
    }

    @Test func convertHandlesSignsWhitespaceAndExponent() {
        #expect(DecimalMarkConverter.convert(" -1,5e3 ", from: ",", to: ".") == " -1.5e3 ")
        #expect(DecimalMarkConverter.convert("+0,25", from: ",", to: ".") == "+0.25")
    }

    @Test func convertLeavesOtherTextUntouched() {
        #expect(DecimalMarkConverter.convert("1,2.3", from: ",", to: ".") == "1,2.3")
        #expect(DecimalMarkConverter.convert("", from: ",", to: ".") == "")
        #expect(DecimalMarkConverter.convert("one,two", from: ",", to: ".") == "one,two")
    }

    @Test func convertWithEmptySourceMarkIsNoOp() {
        #expect(DecimalMarkConverter.convert("1.5", from: "", to: ",") == "1.5")
        #expect(DecimalMarkConverter.convert("abc", from: "", to: ",") == "abc")
    }

    @Test func convertWithIdenticalMarksIsNoOp() {
        #expect(DecimalMarkConverter.convert("1.5", from: ".", to: ".") == "1.5")
    }
}
