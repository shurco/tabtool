import Foundation
import Testing
import TabToolKit
@testable import TabTool

@Suite @MainActor struct RevertCoordinatorTests {
    @Test func cleanDocumentRevertsWithoutConfirmation() async throws {
        let url = try tempFile("a,b\nc,d\n")
        defer { try? FileManager.default.removeItem(at: url) }
        let doc = CSVDocument(data: try Data(contentsOf: url), fileURL: url,
                              configuration: .init(), undoManager: UndoManager())
        await doc.load()

        var asked = 0
        await RevertCoordinator.run(for: doc) { _ in
            asked += 1
            return false
        }

        #expect(asked == 0)
        #expect(doc.table?.cell(row: 0, column: 0) == "a")
    }

    @Test func dirtyRevertCanBeCancelledOrConfirmed() async throws {
        let url = try tempFile("a,b\nc,d\n")
        defer { try? FileManager.default.removeItem(at: url) }
        let um = UndoManager()
        let doc = CSVDocument(data: try Data(contentsOf: url), fileURL: url,
                              configuration: .init(), undoManager: um)
        await doc.load()
        doc.setCell("edited", row: 0, column: 0)
        #expect(doc.hasUnsavedChanges())

        var asked = 0
        await RevertCoordinator.run(for: doc) { _ in
            asked += 1
            return false
        }
        #expect(asked == 1)
        #expect(doc.table?.cell(row: 0, column: 0) == "edited")

        await RevertCoordinator.run(for: doc) { _ in
            asked += 1
            return true
        }
        #expect(asked == 2)
        #expect(doc.table?.cell(row: 0, column: 0) == "a")
        #expect(!doc.hasUnsavedChanges())

        // Revert is one undoable structural action.
        um.undo()
        #expect(doc.table?.cell(row: 0, column: 0) == "edited")
    }

    @Test func revertRequiresAFileBackedDocument() async {
        let doc = CSVDocument(newDocumentConfiguration: { CSVConfiguration() })
        await doc.load()
        #expect(!doc.canRevertToSaved)
        await #expect(throws: DocumentError.notReady) {
            try await doc.revertToSaved()
        }
    }
}
