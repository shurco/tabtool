import AppKit

/// Holds quitting until large files with unsaved edits are saved or discarded.
///
/// The system reviews in-memory documents as before. Large documents are never
/// dirty for the system (see `LargeDocumentWindow`), so they are reviewed here.
final class AppDelegate: NSObject, NSApplicationDelegate {
    func applicationShouldTerminate(_ sender: NSApplication) -> NSApplication.TerminateReply {
        guard LargeDocumentWindow.needsReview else { return .terminateNow }
        Task {
            sender.reply(toApplicationShouldTerminate: await LargeDocumentWindow.reviewAllBeforeQuit())
        }
        return .terminateLater
    }
}
