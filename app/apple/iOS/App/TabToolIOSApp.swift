import SwiftUI

@main
struct TabToolIOSApp: App {
    var body: some Scene {
        DocumentGroup(newDocument: { CSVDocument() }) { configuration in
            DocumentView(document: configuration.document,
                         fileURL: configuration.fileURL)
        }
        .commands {
            AppCommands()
        }
    }
}
