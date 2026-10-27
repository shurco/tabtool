import AppKit
import Combine
import TabToolKit

/// Save, Save As, the dirty dot and the close and quit prompts for the window
/// of a large file.
///
/// A large document is never dirty for the system (its edits are not on the
/// window's undo manager), so the system neither autosaves it, which would
/// rewrite the whole file, nor asks before closing. This object takes over:
/// it stands in as the window's delegate (forwarding everything else to the
/// delegate it replaced), shows the dirty dot, asks Save / Don't Save / Cancel
/// before the window closes or the app quits, and runs the streaming save.
/// Save reaches it through the grid (`CSVTableView`), which it keeps focused
/// when a save starts from the keyboard or the menu bar.
@MainActor
final class LargeDocumentWindow: NSObject, NSWindowDelegate {
    enum Decision { case save, discard, cancel }
    typealias Ask = @MainActor (NSWindow) async -> Decision

    /// The live windows, for the quit review.
    private static let all = NSHashTable<LargeDocumentWindow>.weakObjects()

    let document: CSVDocument
    private weak var window: NSWindow?
    /// The replaced delegate. `nonisolated(unsafe)` so the forwarding
    /// overrides can read it; it is only set in `init`.
    nonisolated(unsafe) private weak var original: NSWindowDelegate?
    private weak var grid: NSView?
    private let ask: Ask
    private var closeApproved = false
    private var changes: AnyCancellable?
    // `nonisolated(unsafe)` lets the nonisolated deinit remove them; they are
    // set once on the main actor.
    nonisolated(unsafe) private var menuObserver: Any?
    nonisolated(unsafe) private var keyMonitor: Any?

    init(window: NSWindow, document: CSVDocument, grid: NSView, ask: @escaping Ask = askToSave) {
        self.window = window
        self.document = document
        self.grid = grid
        self.ask = ask
        original = window.delegate
        super.init()
        window.delegate = self
        Self.all.add(self)
        changes = document.objectWillChange.sink { [weak self] _ in
            Task { @MainActor in self?.refreshEdited() }
        }
        observeSaveRequests()
        refreshEdited()
    }

    deinit {
        menuObserver.map(NotificationCenter.default.removeObserver)
        keyMonitor.map(NSEvent.removeMonitor)
    }

    /// Gives the window its own delegate back.
    func detach() {
        if window?.delegate === self { window?.delegate = original }
        Self.all.remove(self)
    }

    // MARK: Delegate forwarding

    override func responds(to selector: Selector!) -> Bool {
        super.responds(to: selector) || original?.responds(to: selector) == true
    }

    override func forwardingTarget(for selector: Selector!) -> Any? {
        original?.responds(to: selector) == true ? original : nil
    }

    // MARK: Closing and quitting

    func windowWillClose(_ notification: Notification) {
        original?.windowWillClose?(notification)
        Self.all.remove(self)
    }

    func windowShouldClose(_ sender: NSWindow) -> Bool {
        guard !closeApproved, document.hasUnsavedLargeEdits || document.isSaving else {
            return original?.windowShouldClose?(sender) ?? true
        }
        Task {
            guard await reviewUnsavedChanges() else { return }
            closeApproved = true
            sender.performClose(nil)
        }
        return false
    }

    /// Settles unsaved edits before the window closes or the app quits: waits
    /// for a save in flight, then asks. Returns false when the user cancels or
    /// the save fails.
    func reviewUnsavedChanges() async -> Bool {
        await document.waitForLargeSave()
        guard document.hasUnsavedLargeEdits, let window else { return true }
        switch await ask(window) {
        case .save: return await save(to: nil)
        case .discard: return true
        case .cancel: return false
        }
    }

    /// Whether any large file has edits that are not saved, or is being saved.
    static var needsReview: Bool {
        all.allObjects.contains { $0.document.hasUnsavedLargeEdits || $0.document.isSaving }
    }

    /// Reviews every large file in turn; false stops quitting.
    static func reviewAllBeforeQuit() async -> Bool {
        for session in all.allObjects {
            guard await session.reviewUnsavedChanges() else { return false }
        }
        return true
    }

    // MARK: Saving

    var canSave: Bool { document.largeStatus == .ready }

    func save() {
        Task { await save(to: nil) }
    }

    func saveAs() {
        guard let window else { return }
        let panel = NSSavePanel()
        panel.directoryURL = document.fileURL?.deletingLastPathComponent()
        panel.nameFieldStringValue = document.fileURL?.lastPathComponent ?? ""
        panel.beginSheetModal(for: window) { response in
            guard response == .OK, let url = panel.url else { return }
            Task { await self.save(to: url) }
        }
    }

    /// Runs the streaming save, tells the system document where its file now
    /// is, and reports a failure. Returns whether the file was written.
    @discardableResult
    private func save(to destination: URL?) async -> Bool {
        do {
            try await document.saveLargeFile(to: destination)
        } catch is CancellationError {
            return false
        } catch {
            present(error)
            return false
        }
        if let window, let systemDocument = NSDocumentController.shared.document(for: window),
           let url = document.fileURL {
            systemDocument.fileURL = url
            systemDocument.fileModificationDate = try? url.resourceValues(
                forKeys: [.contentModificationDateKey]).contentModificationDate
        }
        return true
    }

    // MARK: Window state

    private func refreshEdited() {
        window?.isDocumentEdited = document.hasUnsavedLargeEdits
    }

    /// Puts the grid back in the responder chain, so Save reaches it and not
    /// the system document.
    func focusGrid() {
        guard let window, let grid,
              !((window.firstResponder as? NSView)?.isDescendant(of: grid) ?? false) else { return }
        window.makeFirstResponder(grid)
    }

    /// Focuses the grid before a Command-S key equivalent and when the menu
    /// bar opens, wherever the focus was.
    private func observeSaveRequests() {
        menuObserver = NotificationCenter.default.addObserver(
            forName: NSMenu.didBeginTrackingNotification, object: nil, queue: .main
        ) { [weak self] _ in
            MainActor.assumeIsolated {
                guard let self, self.window?.isKeyWindow == true else { return }
                self.focusGrid()
            }
        }
        keyMonitor = NSEvent.addLocalMonitorForEvents(matching: .keyDown) { [weak self] event in
            if let self, event.window === self.window, event.modifierFlags.contains(.command),
               event.charactersIgnoringModifiers?.lowercased() == "s" {
                self.focusGrid()
            }
            return event
        }
    }

    private static func askToSave(_ window: NSWindow) async -> Decision {
        let alert = NSAlert()
        alert.messageText = String(localized: "Do you want to save the changes made to “\(window.title)”?")
        alert.informativeText = String(localized: "Your changes will be lost if you don’t save them.")
        alert.addButton(withTitle: String(localized: "Save"))
        alert.addButton(withTitle: String(localized: "Don’t Save"))
        alert.addButton(withTitle: String(localized: "Cancel"))
        switch await alert.beginSheetModal(for: window) {
        case .alertFirstButtonReturn: return .save
        case .alertSecondButtonReturn: return .discard
        default: return .cancel
        }
    }

    private func present(_ error: Error) {
        guard let window else { return }
        let alert = NSAlert()
        alert.alertStyle = .warning
        alert.messageText = String(localized: "The file could not be saved")
        alert.informativeText = FormatErrorText.alertMessage(for: error)
        alert.beginSheetModal(for: window)
    }
}
