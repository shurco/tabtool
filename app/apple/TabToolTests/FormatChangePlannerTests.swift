import Testing
import TabToolKit
@testable import TabTool

@Suite struct FormatChangePlannerTests {
    @Test func onlyDecimalChangesPlanDecimal() {
        var new = CSVConfiguration()
        new.decimalMark = ","
        #expect(FormatChangePlanner.plan(from: .init(), to: new) == .decimalMark(","))
    }

    @Test func onlyHeaderChangesPlanHeader() {
        var new = CSVConfiguration()
        new.firstRowAsHeader = true
        #expect(FormatChangePlanner.plan(from: .init(), to: new) == .header(true))
    }

    @Test func separatorChangePlansReparse() {
        var new = CSVConfiguration()
        new.columnSeparator = ";"
        #expect(FormatChangePlanner.plan(from: .init(), to: new) == .reparse(new))
    }

    @Test func identicalConfigurationPlansNone() {
        #expect(FormatChangePlanner.plan(from: .init(), to: .init()) == .none)
    }

    @Test func encodingChangePlansReparse() {
        var new = CSVConfiguration()
        new.encoding = .windowsCP1252
        #expect(FormatChangePlanner.plan(from: .init(), to: new) == .reparse(new))
    }

    @Test func quoteStyleChangePlansReparse() {
        var new = CSVConfiguration()
        new.escapeCharacter = "\\"
        #expect(FormatChangePlanner.plan(from: .init(), to: new) == .reparse(new))
    }

    @Test func parsingChangeWinsOverHeaderChange() {
        var new = CSVConfiguration()
        new.columnSeparator = "\t"
        new.firstRowAsHeader = true
        #expect(FormatChangePlanner.plan(from: .init(), to: new) == .reparse(new))
    }
}
