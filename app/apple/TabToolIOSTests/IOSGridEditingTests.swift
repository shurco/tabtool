import Foundation
import Testing
import TabToolKit
@testable import TabTool

@MainActor
@Suite struct IOSGridEditingTests {
    private func document() async -> CSVDocument {
        let document = CSVDocument(
            data: Data("1,2\n3,4\n".utf8),
            configuration: CSVConfiguration(firstRowAsHeader: false)
        )
        await document.load()
        return document
    }

    @Test func tappingACellSelectsItsModelRow() async {
        let document = await document()
        let controller = GridController()
        controller.document = document
        controller.selectCell(displayRow: 1, column: 0)
        #expect(document.selection.rows == IndexSet(integer: 1))
        #expect(document.selection.columns.isEmpty)
        #expect(controller.focus == GridFocus(displayRow: 1, column: 0))
    }

    @Test func tappingAHeaderSelectsTheColumn() async {
        let document = await document()
        let controller = GridController()
        controller.document = document
        controller.selectColumn(1)
        #expect(document.selection.columns == IndexSet(integer: 1))
        #expect(document.selection.rows.isEmpty)
    }

    @Test func commitWritesTheCell() async {
        let document = await document()
        let controller = GridController()
        controller.document = document
        controller.isEditable = true
        controller.selectCell(displayRow: 0, column: 1)
        controller.commitEditing("9")
        #expect(document.table?.cell(row: 0, column: 1) == "9")
    }

    @Test func readOnlyGridDoesNotCommit() async {
        let document = await document()
        let controller = GridController()
        controller.document = document
        controller.isEditable = false
        controller.selectCell(displayRow: 0, column: 0)
        controller.commitEditing("9")
        #expect(document.table?.cell(row: 0, column: 0) == "1")
    }

    @Test func tabWrapsToTheNextRow() async {
        let document = await document()
        let controller = GridController()
        controller.document = document
        controller.selectCell(displayRow: 0, column: 1)
        controller.moveFocus(rowDelta: 0, columnDelta: 1)
        #expect(controller.focus == GridFocus(displayRow: 1, column: 0))
    }
}
