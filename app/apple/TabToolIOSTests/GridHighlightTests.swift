import Foundation
import Testing
import UIKit
import TabToolKit
@testable import TabTool

@MainActor @Suite struct GridHighlightTests {
    private func document() async -> CSVDocument {
        let document = CSVDocument(data: Data("1,2\n3,4\n".utf8),
                                   configuration: CSVConfiguration(firstRowAsHeader: false))
        await document.load()
        return document
    }

    @Test func focusBeatsSelection() async {
        let document = await document()
        let controller = GridController()
        controller.document = document
        controller.selectCell(displayRow: 0, column: 0)
        #expect(controller.highlight(displayRow: 0, column: 0) == .focused)
    }

    @Test func aSelectedRowHighlightsEveryColumn() async {
        let document = await document()
        let controller = GridController()
        controller.document = document
        controller.selectCell(displayRow: 0, column: 0)
        #expect(controller.highlight(displayRow: 0, column: 1) == .selected)
        #expect(controller.highlight(displayRow: 1, column: 1) == .none)
    }

    @Test func aSelectedColumnHighlightsEveryRow() async {
        let document = await document()
        let controller = GridController()
        controller.document = document
        controller.selectColumn(1)
        #expect(controller.highlight(displayRow: 1, column: 1) == .selected)
        #expect(controller.highlight(displayRow: 1, column: 0) == .none)
    }

    @Test func focusedCellDrawsATwoPointBorder() {
        let cell = GridDataCell(frame: CGRect(x: 0, y: 0, width: 120, height: 44))
        cell.configure(text: "1", alignment: .left, row: 0, alternating: false,
                       rowHeight: 44, highlight: .focused)
        #expect(cell.contentView.layer.borderWidth == 2)
        #expect(cell.contentView.backgroundColor == .systemBackground)
    }

    @Test func reusingTheCellClearsTheFocusBorder() {
        let cell = GridDataCell(frame: CGRect(x: 0, y: 0, width: 120, height: 44))
        cell.configure(text: "1", alignment: .left, row: 0, alternating: false,
                       rowHeight: 44, highlight: .focused)
        cell.configure(text: "2", alignment: .left, row: 0, alternating: false,
                       rowHeight: 44, highlight: .selected)
        #expect(cell.contentView.layer.borderWidth == 0)
        #expect(cell.contentView.backgroundColor == UIColor.tintColor.withAlphaComponent(0.10))
    }
}
