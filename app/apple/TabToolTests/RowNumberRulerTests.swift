import AppKit
import Foundation
import Testing
import TabToolKit
@testable import TabTool

/// The row number gutter shows each displayed row's model row number.
@Suite @MainActor struct RowNumberRulerTests {
    private func makeRuler(
        _ csv: String
    ) async -> (CSVDocument, CSVGridCoordinator, RowNumberRuler) {
        let doc = CSVDocument(data: Data(csv.utf8), configuration: .init())
        await doc.load()
        let (coordinator, tableView, _) = GridTestSupport.makeGrid(doc)
        let scrollView = NSScrollView()
        scrollView.documentView = tableView
        let ruler = RowNumberRuler(scrollView: scrollView, tableView: tableView)
        return (doc, coordinator, ruler)
    }

    @Test func labelsAreOneBasedRowNumbers() async {
        let (_, coordinator, ruler) = await makeRuler("a\nb\nc\n")
        withExtendedLifetime(coordinator) {
            #expect(ruler.label(forRow: 0) == "1")
            #expect(ruler.label(forRow: 2) == "3")
            #expect(ruler.label(forRow: 3) == nil)
        }
    }

    @Test func filteredRowsKeepTheirModelRowNumbers() async throws {
        let (doc, coordinator, ruler) = await makeRuler("a,1\nb,2\na,3\nc,4\n")
        let columnID = try #require(doc.table?.columnIDs[0])
        doc.rowFilter.criteria = [FilterCriterion(columnID: columnID, op: .equals, value: "a")]
        doc.applyAllFilters()
        GridTestSupport.update(coordinator, from: doc)
        withExtendedLifetime(coordinator) {
            #expect(ruler.label(forRow: 0) == "1")
            #expect(ruler.label(forRow: 1) == "3")
        }
    }

    @Test func widthGrowsWithTheLargestNumber() async {
        let (_, smallCoordinator, small) = await makeRuler("a\nb\n")
        let (_, largeCoordinator, large) = await makeRuler(String(repeating: "x\n", count: 1000))
        withExtendedLifetime((smallCoordinator, largeCoordinator)) {
            small.reload()
            large.reload()
            #expect(large.ruleThickness > small.ruleThickness)
        }
    }
}
