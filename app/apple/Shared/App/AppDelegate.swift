import AppKit

/// Holds quitting until large files with unsaved edits are saved or discarded.
///
/// The system reviews in-memory documents as before. Large documents are never
/// dirty for the system (see `LargeDocumentWindow`), so they are reviewed here.
/// It also starts the new instance when the quit was a relaunch request, and
/// makes launch open an untitled document instead of the open panel.
final class AppDelegate: NSObject, NSApplicationDelegate {
    func applicationWillFinishLaunching(_ notification: Notification) {
        UserDefaults.standard.register(defaults: ["NSShowAppCentricOpenPanelInsteadOfUntitledFile": false])
    }

    func applicationShouldTerminate(_ sender: NSApplication) -> NSApplication.TerminateReply {
        AppRelaunch.quitAttemptStarted()
        guard LargeDocumentWindow.needsReview else { return .terminateNow }
        Task {
            sender.reply(toApplicationShouldTerminate: await LargeDocumentWindow.reviewAllBeforeQuit())
        }
        return .terminateLater
    }

    func applicationWillTerminate(_ notification: Notification) {
        if AppRelaunch.isPending {
            AppRelaunch.launchNewInstance()
        }
    }
}
