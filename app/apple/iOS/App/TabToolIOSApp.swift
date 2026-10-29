import SwiftUI

@main
struct TabToolIOSApp: App {
    @AppStorage(Preferences.Key.appLanguage) private var language = Preferences.Default.appLanguage

    var body: some Scene {
        WindowGroup {
            DocumentLibraryView()
                .environment(\.locale, language.locale ?? .autoupdatingCurrent)
        }
    }
}
