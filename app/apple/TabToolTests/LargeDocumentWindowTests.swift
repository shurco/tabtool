import AppKit
import Foundation
import Testing
import TabToolKit
@testable import TabTool

/// The window delegate SwiftUI would have installed.
private final class OriginalDelegate: NSObject, NSWindowDelegate {
    var allowsClose = true
    var resizes = 0
    func windowShouldClose(_ sender: NSWindow) -> Bool { allowsClose }
    func windowDidResize(_ notification: Notification) { resizes += 1 }
}

@Suite(.serialized) @MainActor struct LargeDocumentWindowTests {
    private func makeWindow() -> NSWindow {
        let window = NSWindow(contentRect: NSRect(x: 0, y: 0, width: 400, height: 300),
                              styleMask: [.titled, .closable], backing: .buffered, defer: true)
        window.isReleasedWhenClosed = false
        return window
    }

    private func open(_ dir: LargeTestDirectory) async throws -> CSVDocument {
        let document = try await openedLargeDocument(dir.csv(rows: 300))
        if document.configuration.firstRowAsHeader { document.setHeaderEnabled(false) }
        return document
    }

    @Test func standsInForTheWindowDelegateUntilDetached() async throws {
        let dir = LargeTestDirectory()
        let document = try await open(dir)
        let window = makeWindow()
        let original = OriginalDelegate()
        window.delegate = original
        let session = LargeDocumentWindow(window: window, document: document, grid: NSView())

        #expect(window.delegate === session)
        NotificationCenter.default.post(name: NSWindow.didResizeNotification, object: window)
        #expect(original.resizes == 1)
        original.allowsClose = false
        #expect(!session.windowShouldClose(window))

        session.detach()
        #expect(window.delegate === original)
    }

    @Test func dirtyDotFollowsTheEdits() async throws {
        let dir = LargeTestDirectory()
        let document = try await open(dir)
        let window = makeWindow()
        let session = LargeDocumentWindow(window: window, document: document, grid: NSView())
        defer { session.detach() }
        #expect(!window.isDocumentEdited)

        document.setCell("X", row: 1, column: 1)
        try await Task.sleep(for: .milliseconds(20))
        #expect(window.isDocumentEdited)
        document.undoManager?.undo()
        try await Task.sleep(for: .milliseconds(20))
        #expect(!window.isDocumentEdited)
    }

    @Test func reviewAsksOnlyWhenThereAreUnsavedEdits() async throws {
        let dir = LargeTestDirectory()
        let document = try await open(dir)
        var asked = 0
        var decision = LargeDocumentWindow.Decision.cancel
        let session = LargeDocumentWindow(window: makeWindow(), document: document, grid: NSView()) { _ in
            asked += 1
            return decision
        }
        defer { session.detach() }

        #expect(await session.reviewUnsavedChanges())
        #expect(asked == 0)

        document.setCell("X", row: 1, column: 1)
        #expect(!(await session.reviewUnsavedChanges()))
        #expect(!(await LargeDocumentWindow.reviewAllBeforeQuit()))
        #expect(document.hasUnsavedLargeEdits)

        decision = .discard
        #expect(await session.reviewUnsavedChanges())
        #expect(document.hasUnsavedLargeEdits)

        decision = .save
        #expect(await session.reviewUnsavedChanges())
        #expect(!document.hasUnsavedLargeEdits)
        #expect(document.largeSource?.cell(row: 1, column: 1) == "X")
        #expect(asked == 4)
    }

    @Test func closingADirtyWindowWaitsForTheReview() async throws {
        let dir = LargeTestDirectory()
        let document = try await open(dir)
        let window = makeWindow()
        let session = LargeDocumentWindow(window: window, document: document, grid: NSView()) { _ in .cancel }
        defer { session.detach() }
        document.setCell("X", row: 1, column: 1)
        #expect(!session.windowShouldClose(window))
    }

    @Test func savingFocusesTheGridFirst() async throws {
        let dir = LargeTestDirectory()
        let document = try await open(dir)
        let window = makeWindow()
        let grid = NSTextView()
        let other = NSTextView()
        window.contentView?.addSubview(grid)
        window.contentView?.addSubview(other)
        let session = LargeDocumentWindow(window: window, document: document, grid: grid)
        defer { session.detach() }

        window.makeFirstResponder(other)
        session.focusGrid()
        #expect(window.firstResponder === grid)
    }

    @Test func gridAnswersSaveOnlyForLargeFiles() async throws {
        let dir = LargeTestDirectory()
        let large = try await open(dir)
        let small = CSVDocument(data: Data("a,b\n".utf8), configuration: .init())
        await small.load()
        let saveAction = #selector(NSDocument.save(_:))
        let duplicate = NSMenuItem(title: "Duplicate", action: #selector(NSDocument.duplicate(_:)),
                                   keyEquivalent: "")

        for (document, answers) in [(large, true), (small, false)] {
            let (coordinator, tableView, _) = GridTestSupport.makeGrid(document)
            let window = makeWindow()
            window.contentView?.addSubview(tableView)
            withExtendedLifetime(coordinator) {
                #expect(tableView.responds(to: saveAction) == answers)
                if answers { #expect(!tableView.validateUserInterfaceItem(duplicate)) }
            }
            tableView.removeFromSuperview()
        }
    }
}
