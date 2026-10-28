import AppKit

/// Quits the app and starts a fresh instance once the quit is certain.
///
/// The quit goes through the normal flow, so unsaved documents are reviewed
/// first. The new instance starts only from `applicationWillTerminate`; a
/// cancelled quit therefore leaves the app running.
@MainActor
enum AppRelaunch {
    /// Set by `request`, taken over by the next quit attempt.
    private static var requested = false

    /// Whether the quit in progress was started by `request`.
    private(set) static var isPending = false

    static func request(quit: () -> Void = { NSApp.terminate(nil) }) {
        requested = true
        quit()
    }

    /// Called at the start of every quit attempt. A later quit that does not
    /// come from `request` (the user cancelled the first one) does not relaunch.
    static func quitAttemptStarted() {
        isPending = requested
        requested = false
    }

    /// Starts another instance of this app; `-n` because this one is still alive.
    static func launchNewInstance() {
        let process = Process()
        process.executableURL = URL(fileURLWithPath: "/usr/bin/open")
        process.arguments = ["-n", Bundle.main.bundlePath]
        try? process.run()
    }
}
