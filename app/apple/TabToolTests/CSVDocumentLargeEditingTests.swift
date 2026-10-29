import Foundation
import Testing
import TabToolKit
#if os(macOS)
import AppKit
#endif
@testable import TabTool

@Suite @MainActor struct CSVDocumentLargeEditingTests {
    /// A ready large document over `rows` data rows; the file has no header row.
    private func open(rows: Int = 300, in dir: LargeTestDirectory) async throws -> CSVDocument {
        let document = try await openedLargeDocument(dir.csv(rows: rows))
        if document.configuration.firstRowAsHeader { document.setHeaderEnabled(false) }
        return document
    }

    @Test func editRegistersUndoAndRedo() async throws {
        let dir = LargeTestDirectory()
        let document = try await open(in: dir)
        let source = try #require(document.largeSource)
        document.setCell("X", row: 2, column: 1)
        #expect(source.cell(row: 2, column: 1) == "X")
        #expect(source.overlay.count == 1)

        document.undoManager?.undo()
        #expect(source.cell(row: 2, column: 1) == "v2")
        #expect(source.overlay.count == 0)
        document.undoManager?.redo()
        #expect(source.cell(row: 2, column: 1) == "X")
        #expect(source.overlay.count == 1)
    }

    @Test func editBackToOriginalIsClean() async throws {
        let dir = LargeTestDirectory()
        let document = try await open(in: dir)
        document.setCell("X", row: 0, column: 0)
        #expect(await document.hasUnsavedChangesAsync())
        document.setCell("r0", row: 0, column: 0)
        #expect(!(await document.hasUnsavedChangesAsync()))
        #expect(!document.hasUnsavedChanges())
    }

    @Test func editRejectedWhenTheFileChangedAndRegistersNoUndo() async throws {
        let dir = LargeTestDirectory()
        let url = try dir.csv(rows: 300)
        let document = try await openedLargeDocument(url)
        try Data("short".utf8).write(to: url)
        document.setCell("X", row: 0, column: 0)
        #expect(document.notice == .editRejected(.rowNotEditable))
        #expect(document.undoManager?.canUndo == false)
        #expect(document.largeSource?.overlay.isEmpty == true)
    }

    @Test func revertDropsOverlayAndUndo() async throws {
        let dir = LargeTestDirectory()
        let document = try await open(in: dir)
        document.setCell("X", row: 1, column: 1)
        #expect(document.canRevertToSaved)
        try await document.revertToSaved()
        #expect(document.largeSource?.overlay.isEmpty == true)
        #expect(document.largeSource?.cell(row: 1, column: 1) == "v1")
        #expect(document.undoManager?.canUndo == false)
    }

    @Test func reparseWithDirtyOverlayRequiresConfirmation() async throws {
        let dir = LargeTestDirectory()
        let document = try await open(in: dir)
        document.setCell("X", row: 1, column: 1)
        let before = document.largeSource
        var unquoted = document.configuration
        unquoted.quoteCharacter = ""
        unquoted.escapeCharacter = ""
        await #expect(throws: DocumentError.requiresConfirmation) {
            try await document.applyParsingConfiguration(unquoted)
        }
        #expect(document.largeSource === before)
        try await document.applyParsingConfiguration(unquoted, force: true)
        #expect(document.configuration.quoteCharacter == "")
        #expect(document.largeSource !== before)
        try await waitForIndexing(of: document)
        #expect(document.largeSource?.cell(row: 1, column: 2) == "\"q 1\"")
        #expect(document.largeSource?.overlay.isEmpty == true)
        #expect(document.undoManager?.canUndo == false)
    }

    @Test func reparseToUTF16IsRefusedAndChangesNothing() async throws {
        let dir = LargeTestDirectory()
        let document = try await open(in: dir)
        let before = document.largeSource
        var utf16 = document.configuration
        utf16.encoding = .utf16
        await #expect(throws: CSVError.unsupportedLargeFileEncoding) {
            try await document.applyParsingConfiguration(utf16, force: true)
        }
        #expect(document.largeSource === before)
        #expect(document.configuration.encoding == .utf8)
    }

    @Test func headerToggleChangesRowMappingAndIsUndoable() async throws {
        let dir = LargeTestDirectory()
        let document = try await open(rows: 50, in: dir)
        let source = try #require(document.largeSource)
        #expect(source.rowCount == 50)
        document.setHeaderEnabled(true)
        #expect(document.configuration.firstRowAsHeader)
        #expect(source.rowCount == 49)
        #expect(source.headers?.first == "r0")
        document.undoManager?.undo()
        #expect(!document.configuration.firstRowAsHeader)
        #expect(source.rowCount == 50)
    }

    @Test func undoSurvivesAHeaderToggle() async throws {
        let dir = LargeTestDirectory()
        let document = try await open(rows: 50, in: dir)
        document.setCell("X", row: 3, column: 0)
        document.setHeaderEnabled(true)
        document.undoManager?.undo()
        document.undoManager?.undo()
        #expect(document.largeSource?.overlay.isEmpty == true)
        #expect(document.largeSource?.cell(row: 3, column: 0) == "r3")
    }

    @Test func detectFormatReadsTheSample() async throws {
        let dir = LargeTestDirectory()
        let document = try await open(in: dir)
        let detected = try await document.detectFormatFromDisk()
        #expect(detected.columnSeparator == ",")
    }

    @Test func copiesSelectedRowsAsTSV() async throws {
        let dir = LargeTestDirectory()
        let document = try await open(in: dir)
        document.selection = GridSelection(rows: IndexSet([1, 2]))
        #expect(document.copySelection() == "r1\tv1\tq 1\nr2\tv2\tq 2")
    }

    @Test func copyOverTheCellLimitIsRefused() async throws {
        let dir = LargeTestDirectory()
        let document = try await open(rows: 340_000, in: dir)
        document.selection = GridSelection(rows: IndexSet(0..<340_000))
        #expect(document.copySelection() == nil)
        #expect(document.notice == .copyLimitExceeded)
    }

    #if os(macOS)
    @Test func structuralActionsAreInertButCellsStayEditable() async throws {
        let dir = LargeTestDirectory()
        let document = try await open(in: dir)
        let (coordinator, tableView, _) = GridTestSupport.makeGrid(document)
        withExtendedLifetime(coordinator) {
            coordinator.update(presenter: GridPresenter(table: document.largeSource!,
                                                        configuration: document.configuration),
                               isEditable: true, allowsStructuralEdits: false)
            #expect(coordinator.isEditable)
            #expect(!coordinator.canPaste)
            #expect(!coordinator.canDeleteSelection)
            #expect(!coordinator.headerMenu(forColumn: 0).items[0].isEnabled)
            #expect(!coordinator.rowMenu().items[0].isEnabled)
            #expect(tableView.undoManager === document.undoManager)
        }
    }

    @Test func accessibilityExposesOnlyVisibleRowsOfALargeFile() async throws {
        let dir = LargeTestDirectory()
        let document = try await open(rows: 100_000, in: dir)
        let (coordinator, tableView, _) = GridTestSupport.makeGrid(document)
        let scrollView = NSScrollView(frame: NSRect(x: 0, y: 0, width: 600, height: 200))
        scrollView.documentView = tableView
        withExtendedLifetime(coordinator) {
            coordinator.update(presenter: GridPresenter(table: document.largeSource!,
                                                        configuration: document.configuration),
                               isEditable: true, allowsStructuralEdits: false)
            #expect(tableView.numberOfRows == 100_000)
            #expect((tableView.accessibilityRows() ?? []).count < 100)
            #expect((tableView.accessibilityChildren() ?? []).count < 120)
            // Accessibility clients reach NSTableView through the legacy
            // attribute API, which builds every row unless it is answered too.
            for attribute in [NSAccessibility.Attribute.children, .rows] {
                let value = tableView.perform(NSSelectorFromString("accessibilityAttributeValue:"),
                                              with: attribute.rawValue)?.takeUnretainedValue()
                #expect(((value as? NSArray)?.count ?? .max) < 120)
            }
        }
    }
    #endif
}
