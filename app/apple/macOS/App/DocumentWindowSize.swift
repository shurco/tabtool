import AppKit

/// Remembers the size of document windows, so a document opens at the size
/// the user last gave a document window, not at the default size.
///
/// One size is shared by all documents. `TabToolApp` reads it as the default
/// size of the document scene; this type saves it whenever a document window
/// changes size. The stored size is the content size, which is what the
/// scene's `defaultSize` sets.
enum DocumentWindowSize {
    /// The key strings are persisted data and must not change.
    static let widthKey = "documentWindowWidth"
    static let heightKey = "documentWindowHeight"

    static let defaultWidth = 900.0
    static let defaultHeight = 450.0

    /// Starts saving the size of every document window the user resizes.
    ///
    /// A live resize is saved once at its end, so the scene is not rebuilt
    /// on every step of a drag. Zoom and other programmatic resizes are saved
    /// at once. The observers live for the whole app run.
    static func startObserving() {
        let center = NotificationCenter.default
        _ = center.addObserver(forName: NSWindow.didResizeNotification, object: nil, queue: .main) { note in
            nonisolated(unsafe) let window = note.object as? NSWindow
            MainActor.assumeIsolated {
                guard let window, !window.inLiveResize else { return }
                save(window)
            }
        }
        _ = center.addObserver(forName: NSWindow.didEndLiveResizeNotification, object: nil, queue: .main) { note in
            nonisolated(unsafe) let window = note.object as? NSWindow
            MainActor.assumeIsolated {
                guard let window else { return }
                save(window)
            }
        }
    }

    @MainActor
    private static func save(_ window: NSWindow) {
        // Only document windows; a full-screen size is not a window size.
        guard window.windowController?.document != nil,
              !window.styleMask.contains(.fullScreen) else { return }
        let size = window.contentRect(forFrameRect: window.frame).size
        let defaults = UserDefaults.standard
        defaults.set(Double(size.width), forKey: widthKey)
        defaults.set(Double(size.height), forKey: heightKey)
    }
}
