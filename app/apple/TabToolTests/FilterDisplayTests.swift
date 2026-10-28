import Foundation
import Testing
import TabToolKit
@testable import TabTool

@Suite @MainActor struct FilterDisplayTests {
    @Test func symbolOperatorsShowTheirSymbol() {
        let symbols: [FilterOperator: String] = [
            .equals: "=", .notEquals: "≠", .greaterThan: ">", .lessThan: "<",
            .greaterOrEqual: "≥", .lessOrEqual: "≤",
        ]
        for (filterOperator, symbol) in symbols {
            #expect(filterOperator.title == symbol)
        }
    }

    @Test func everyOperatorHasAUniqueTitle() {
        let titles = FilterOperator.allCases.map(\.title)
        #expect(Set(titles).count == titles.count)
    }

    @Test func wordOperatorsAreTranslated() throws {
        let path = try #require(Bundle.main.path(forResource: "fr", ofType: "lproj"))
        let french = try #require(Bundle(path: path))
        let words = FilterOperator.allCases.map(\.title).filter { $0.contains(where: \.isLetter) }
        #expect(words.count == 7)
        for title in words {
            #expect(french.localized(title) != title, "\(title) is not translated")
        }
    }

    @Test func columnsUseHeadersOrGeneratedNames() {
        let withHeaders = CSVTable(rows: [["1", "2"]], headers: ["id", "name"])
        let named = FilterColumn.columns(of: withHeaders, configuration: .init())
        #expect(named.map(\.name) == ["id", "name"])
        #expect(named.map(\.id) == withHeaders.columnIDs)

        let headerless = CSVTable(rows: [["1", "2", "3"]], headers: nil)
        #expect(FilterColumn.columns(of: headerless, configuration: .init()).map(\.name) == ["A", "B", "C"])
    }

    @Test func statusTextShowsVisibleOfTotalWhileFiltering() {
        let filtered = DocumentStatusBar.rowsText(total: 4, visible: 2)
        #expect(filtered == "2 of 4 rows")
    }

    @Test func statusTextWithoutFilterIsThePlainRowCount() {
        #expect(DocumentStatusBar.rowsText(total: 4, visible: nil) == "4 rows")
        #expect(DocumentStatusBar.rowsText(total: 1, visible: nil) == "1 row")
    }
}
