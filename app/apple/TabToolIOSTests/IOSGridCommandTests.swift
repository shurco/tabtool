import Foundation
import Testing
import UIKit
import TabToolKit
@testable import TabTool

@MainActor
@Suite struct IOSGridCommandTests {
    private func document(_ csv: String = "b,2\na,1\n") async -> CSVDocument {
        let document = CSVDocument(
            data: Data(csv.utf8),
            configuration: CSVConfiguration(firstRowAsHeader: false)
        )
        await document.load()
        return document
    }

    private func controller(for document: CSVDocument) -> GridController {
        let controller = GridController()
        controller.document = document
        controller.allowsStructuralEdits = true
        controller.isEditable = true
        return controller
    }

    @Test func addRowBelowInsertsUnderTheSelection() async {
        let document = await document()
        let controller = controller(for: document)
        controller.selectCell(displayRow: 0, column: 0)
        controller.addRowBelow()
        #expect(document.table?.rowCount == 3)
        #expect(document.table?.cell(row: 1, column: 0) == "")
    }

    @Test func copyAndPasteRoundTripTheSelection() async {
        let document = await document()
        let controller = controller(for: document)
        controller.selectCell(displayRow: 0, column: 0)
        #expect(controller.copySelection())
        #expect(UIPasteboard.general.string?.contains("b") == true)
        #expect(controller.paste())
        #expect(document.table?.rowCount == 3)
    }

    @Test func sortAndRenameUseTheSelectedColumn() async {
        let document = await document()
        let controller = controller(for: document)
        controller.selectColumn(0)
        controller.sortFocusedColumn(.ascending)
        #expect(document.table?.cell(row: 0, column: 0) == "a")
        controller.renameFocusedColumn(to: "Name")
        #expect(document.table?.headers?.first == "Name")
    }

    @Test func movingARowReordersIt() async {
        let document = await document()
        let controller = controller(for: document)
        controller.moveRow(fromDisplayRow: 0, beforeDisplayRow: 2)
        #expect(document.table?.cell(row: 0, column: 0) == "a")
        #expect(document.table?.cell(row: 1, column: 0) == "b")
    }

    @Test func anActiveFilterBlocksRowMoves() async {
        let document = await document()
        document.rowFilter.criteria[0].columnID = document.table?.columnIDs[0]
        document.rowFilter.criteria[0].op = .equals
        document.rowFilter.criteria[0].value = "b"
        document.rowFilter.criteria[0].isEnabled = true
        document.rowFilter.applyAll(to: document.table!, decimalMark: ".")
        let controller = controller(for: document)
        controller.moveRow(fromDisplayRow: 0, beforeDisplayRow: 1)
        #expect(document.table?.cell(row: 0, column: 0) == "b")
    }

    @Test func movingAColumnReordersIt() async {
        let document = await document()
        let controller = controller(for: document)
        controller.moveColumn(from: 0, before: 2)
        #expect(document.table?.cell(row: 0, column: 0) == "2")
        #expect(document.table?.cell(row: 0, column: 1) == "b")
    }

    @Test func aRightwardColumnDropLandsAfterTheTarget() {
        #expect(GridController.dropDestination(from: 0, onto: 1) == 2)
        #expect(GridController.dropDestination(from: 1, onto: 0) == 0)
        #expect(GridController.dropDestination(from: 1, onto: 1) == 1)
    }
}
