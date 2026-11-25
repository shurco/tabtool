import SwiftUI

@main
struct TabToolApp: App {
    @NSApplicationDelegateAdaptor(AppDelegate.self) private var appDelegate
    @AppStorage(Preferences.Key.appLanguage) private var language = Preferences.Default.appLanguage
    @AppStorage(DocumentWindowSize.widthKey) private var windowWidth = DocumentWindowSize.defaultWidth
    @AppStorage(DocumentWindowSize.heightKey) private var windowHeight = DocumentWindowSize.defaultHeight

    var body: some Scene {
        DocumentGroup(newDocument: { CSVDocument() }) { configuration in
            DocumentView(document: configuration.document,
                         fileURL: configuration.fileURL)
                .environment(\.locale, locale)
        }
        .defaultSize(width: windowWidth, height: windowHeight)
        .commands {
            AppCommands()
        }

        Window("About TabTool", id: AboutView.windowID) {
            AboutView()
                .environment(\.locale, locale)
        }
        .windowResizability(.contentSize)

        Settings {
            SettingsView()
                .environment(\.locale, locale)
        }
    }

    private var locale: Locale { language.locale ?? .autoupdatingCurrent }
}
