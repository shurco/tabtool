import SwiftUI

@main
struct TabToolApp: App {
    var body: some Scene {
        DocumentGroup(newDocument: { CSVDocument() }) { configuration in
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
