import SwiftUI

@main
struct TabToolApp: App {
    var body: some Scene {
        DocumentGroup(newDocument: {
            // DocumentGroup creates documents on the main thread; the
            // preconcurrency API does not express that in its signature.
            MainActor.assumeIsolated { CSVDocument() }
        }) { configuration in
            DocumentView(document: configuration.document,
                         fileURL: configuration.fileURL)
        }
        .commands {
            AppCommands()
        }

        Window("About TabTool", id: AboutView.windowID) {
            AboutView()
        }
        .windowResizability(.contentSize)
    }
}
