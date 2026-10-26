import AppKit
import Foundation
import TabToolKit
import Testing
@testable import TabTool

@Suite @MainActor struct GridInteractionTests {
    // MARK: - Document interactions from the brief

    @Test @MainActor func addRowUsesSelectionAndUndoName() async throws {
        let um = UndoManager()
        let doc = CSVDocument(data: Data("0\n1\n".utf8), configuration: .init(),
                              undoManager: um)
        await doc.load()
        doc.selection = GridSelection(rows: IndexSet([1]))
        doc.addRow(at: doc.selection.rows.last! + 1)
        #expect(doc.table?.rowCount == 3)
        #expect(um.undoActionName == "Add Row")
        um.undo()
        #expect(doc.table?.rowCount == 2)
    }

    @Test @MainActor func copyColumnsUsesAllRows() async throws {
        let doc = CSVDocument(data: Data("a,b\nc,d\n".utf8), configuration: .init())
        await doc.load()
        doc.selection = GridSelection(columns: IndexSet([1]))
        #expect(doc.copySelection(locale: Locale(identifier: "en_US")) == "b\nd")
    }

    @Test @MainActor func deleteSelectionPrefersRowsOverColumns() async throws {
        let um = UndoManager()
        let doc = CSVDocument(data: Data("a,b\nc,d\n".utf8), configuration: .init(),
                              undoManager: um)
        await doc.load()
        doc.selection = GridSelection(rows: IndexSet([0]), columns: IndexSet([1]))
        doc.deleteSelection()
        #expect(doc.table?.rowCount == 1)
        #expect(doc.table?.columns.count == 2)
        um.undo()
        #expect(doc.table?.rowCount == 2)
    }

    // MARK: - Copy and paste

    @Test func copyWritesTabularTextAndString() async throws {
        let doc = CSVDocument(data: Data("a,b\nc,d\n".utf8), configuration: .init())
        await doc.load()
        doc.selection = GridSelection(rows: IndexSet([0]))
        let (coordinator, _, _) = GridTestSupport.makeGrid(doc)
        let pasteboard = GridTestSupport.makePasteboard()
        pasteboard.clearContents()

        #expect(coordinator.copySelection(to: pasteboard))
        #expect(pasteboard.string(forType: .tabularText) == "a\tb")
        #expect(pasteboard.string(forType: .string) == "a\tb")
    }

    @Test func copyWithoutSelectionLeavesThePasteboardAlone() async throws {
        let doc = CSVDocument(data: Data("a,b\n".utf8), configuration: .init())
        await doc.load()
        let (coordinator, _, _) = GridTestSupport.makeGrid(doc)
        let pasteboard = GridTestSupport.makePasteboard()
        pasteboard.clearContents()
        pasteboard.setString("kept", forType: .string)

        #expect(!coordinator.copySelection(to: pasteboard))
        #expect(pasteboard.string(forType: .string) == "kept")
    }

    @Test func pasteReadsTheStringAndSelectsInsertedRows() async throws {
        let doc = CSVDocument(data: Data("a,b\n".utf8), configuration: .init())
        await doc.load()
        doc.selection = GridSelection(rows: IndexSet([0]))
        let (coordinator, tableView, _) = GridTestSupport.makeGrid(doc)
        let pasteboard = GridTestSupport.makePasteboard()
        pasteboard.clearContents()
        pasteboard.setString("x\ty", forType: .string)

        #expect(coordinator.paste(from: pasteboard))
        #expect(doc.table?.rowCount == 2)
        #expect(doc.table?.cell(row: 1, column: 0) == "x")
        #expect(doc.table?.cell(row: 1, column: 1) == "y")
        #expect(doc.selection == GridSelection(rows: IndexSet([1])))

        GridTestSupport.update(coordinator, from: doc)
        #expect(tableView.selectedRowIndexes == IndexSet([1]))
    }

    @Test func pasteFallsBackToTabularText() async throws {
        let doc = CSVDocument(data: Data("a,b\n".utf8), configuration: .init())
        await doc.load()
        let (coordinator, _, _) = GridTestSupport.makeGrid(doc)
        let pasteboard = GridTestSupport.makePasteboard()
        pasteboard.clearContents()
        pasteboard.setString("z", forType: .tabularText)

        #expect(coordinator.paste(from: pasteboard))
        #expect(doc.table?.rowCount == 2)
        #expect(doc.table?.cell(row: 1, column: 0) == "z")
    }

    @Test func pasteKeepsAnEmptyStringAsOneCell() async throws {
        let doc = CSVDocument(data: Data("a,b\n".utf8), configuration: .init())
        await doc.load()
        let (coordinator, _, _) = GridTestSupport.makeGrid(doc)
        let pasteboard = GridTestSupport.makePasteboard()
        pasteboard.clearContents()
        pasteboard.setString("", forType: .string)

        #expect(coordinator.paste(from: pasteboard))
        #expect(doc.table?.rowCount == 2)
        #expect(doc.selection == GridSelection(rows: IndexSet([1])))
    }

    @Test func pasteIgnoresAPasteboardWithoutText() async throws {
        let doc = CSVDocument(data: Data("a,b\n".utf8), configuration: .init())
        await doc.load()
        let (coordinator, _, _) = GridTestSupport.makeGrid(doc)
        let pasteboard = GridTestSupport.makePasteboard()
        pasteboard.clearContents()

        #expect(!coordinator.paste(from: pasteboard))
        #expect(doc.table?.rowCount == 1)
    }

    // MARK: - Selection

    @Test func selectAllSelectsEveryRow() async throws {
        let doc = CSVDocument(data: Data("a\nb\n".utf8), configuration: .init())
        await doc.load()
        let (coordinator, tableView, _) = GridTestSupport.makeGrid(doc)
        withExtendedLifetime(coordinator) {
            coordinator.selectAllRows()
        }

        #expect(doc.selection == GridSelection(rows: IndexSet([0, 1])))
        #expect(tableView.selectedRowIndexes == IndexSet([0, 1]))
    }

    @Test func menuValidationFollowsTheSelection() async throws {
        let doc = CSVDocument(data: Data("a,b\n".utf8), configuration: .init())
        await doc.load()
        let (coordinator, tableView, headerView) = GridTestSupport.makeGrid(doc)

        let copyItem = NSMenuItem()
        copyItem.action = #selector(CSVTableView.copy(_:))
        let deleteItem = NSMenuItem()
        deleteItem.action = #selector(CSVTableView.delete(_:))

        withExtendedLifetime(coordinator) {
            #expect(!tableView.validateUserInterfaceItem(copyItem))
            #expect(!tableView.validateUserInterfaceItem(deleteItem))

            headerView.clickColumn(at: 1, modifiers: [])

            #expect(tableView.validateUserInterfaceItem(copyItem))
            #expect(tableView.validateUserInterfaceItem(deleteItem))
        }
    }

    @Test func deleteEnablementRejectsOnlyColumnSelection() async throws {
        let doc = CSVDocument(data: Data("a\nb\n".utf8), configuration: .init())
        await doc.load()
        #expect(!doc.canDeleteSelection)

        doc.selection = GridSelection(rows: IndexSet([0]))
        #expect(doc.canDeleteSelection)

        doc.selection = GridSelection(columns: IndexSet([0]))
        #expect(!doc.canDeleteSelection)

        let twoColumns = CSVDocument(data: Data("a,b\n".utf8), configuration: .init())
        await twoColumns.load()
        twoColumns.selection = GridSelection(columns: IndexSet([0]))
        #expect(twoColumns.canDeleteSelection)
    }

    @Test func deleteMenuIsDisabledForOnlyColumnSelection() async throws {
        let doc = CSVDocument(data: Data("a\nb\n".utf8), configuration: .init())
        await doc.load()
        doc.selection = GridSelection(columns: IndexSet([0]))
        let (coordinator, tableView, _) = GridTestSupport.makeGrid(doc)
        let deleteItem = NSMenuItem()
        deleteItem.action = #selector(CSVTableView.delete(_:))

        withExtendedLifetime(coordinator) {
            #expect(!tableView.validateUserInterfaceItem(deleteItem))
            #expect(!coordinator.deleteSelection())
        }
        #expect(doc.table?.columns.count == 1)
    }

    @Test func deleteKeyRemovesTheSelectedRows() async throws {
        let doc = CSVDocument(data: Data("a\nb\n".utf8), configuration: .init())
        await doc.load()
        doc.selection = GridSelection(rows: IndexSet([0]))
        let (coordinator, tableView, _) = GridTestSupport.makeGrid(doc)
        let event = try #require(NSEvent.keyEvent(
            with: .keyDown, location: .zero, modifierFlags: [], timestamp: 0,
            windowNumber: 0, context: nil, characters: "\u{7F}",
            charactersIgnoringModifiers: "\u{7F}", isARepeat: false, keyCode: 51
        ))

        withExtendedLifetime(coordinator) {
            tableView.keyDown(with: event)
        }

        #expect(doc.table?.rowCount == 1)
        #expect(doc.selection == GridSelection())
    }

    @Test func structuralChangeClearsOutOfRangeSelection() async throws {
        let doc = CSVDocument(data: Data("a\nb\nc\n".utf8), configuration: .init())
        await doc.load()
        doc.selection = GridSelection(rows: IndexSet([2]))

        doc.deleteRows(at: IndexSet([2]))

        #expect(doc.selection == GridSelection())
    }

    @Test func undoClearsSelectionRemovedByPasteUndo() async throws {
        let um = UndoManager()
        let doc = CSVDocument(data: Data("a\nb\n".utf8), configuration: .init(),
                              undoManager: um)
        await doc.load()
        doc.selection = GridSelection(rows: IndexSet([1]))
        doc.pasteTSV("x")
        #expect(doc.table?.rowCount == 3)
        #expect(doc.selection == GridSelection(rows: IndexSet([2])))

        um.undo()

        #expect(doc.table?.rowCount == 2)
        #expect(doc.selection == GridSelection())
    }
}
