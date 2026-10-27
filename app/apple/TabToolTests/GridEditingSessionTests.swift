import AppKit
import Foundation
import TabToolKit
import Testing
@testable import TabTool

/// Verifies that the document asks the grid to commit an active edit before
/// structural mutations. The real AppKit field editor is exercised by
/// `testTypedCellSurvivesAddRowSaveAndReopen` in the UI suite.
@Suite @MainActor struct GridEditingSessionTests {
    private final class RecordingSession: DocumentEditingSession {
        var endEditingCount = 0
        var onEndEditing: (() -> Void)?

        func endEditing() {
            endEditingCount += 1
            onEndEditing?()
        }
    }

    @Test func structuralChangeEndsEditingBeforeTheUndoSnapshot() async {
        let um = UndoManager()
        let doc = CSVDocument(data: Data("a,b\nc,d\n".utf8),
                              configuration: .init(), undoManager: um)
        await doc.load()
        let session = RecordingSession()
        doc.editingSession = session

        // Simulate what the grid's field editor does on commit.
        session.onEndEditing = {
            doc.setCell("typed", row: 0, column: 0)
        }

        doc.addRow(at: 2)

        #expect(session.endEditingCount >= 1)
        #expect(doc.table?.rowCount == 3)
        #expect(doc.table?.cell(row: 0, column: 0) == "typed")

        // Add Row's snapshot was taken after the edit committed, so undoing
        // the row keeps the committed text.
        um.undo()
        #expect(doc.table?.rowCount == 2)
        #expect(doc.table?.cell(row: 0, column: 0) == "typed")
    }

    @Test func revertEndsEditingBeforeReadingTheFile() async throws {
        let url = try tempFile("a,b\nc,d\n")
        defer { try? FileManager.default.removeItem(at: url) }
        let doc = CSVDocument(data: try Data(contentsOf: url), fileURL: url,
                              configuration: .init(), undoManager: UndoManager())
        await doc.load()
        let session = RecordingSession()
        doc.editingSession = session

        try await doc.revertToSaved()

        #expect(session.endEditingCount >= 1)
    }
}
