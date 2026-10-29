import SwiftUI

/// The macOS Settings window: one tab per group of settings.
///
/// Every control writes straight to the stored preference; open documents
/// react to the grid and format bar preferences through their own `@AppStorage`.
/// Each tab form is as tall as its rows, so the window follows the selected tab.
struct SettingsView: View {
    var body: some View {
        TabView {
            Form { GeneralSettings() }
                .fixedSize(horizontal: false, vertical: true)
                .tabItem { Label("General", systemImage: "gearshape") }
            Form { NewDocumentSettings() }
                .fixedSize(horizontal: false, vertical: true)
                .tabItem { Label("New Document", systemImage: "doc") }
            Form { GridSettings() }
                .fixedSize(horizontal: false, vertical: true)
                .tabItem { Label("Grid", systemImage: "tablecells") }
            Form { LargeFileSettings() }
                .fixedSize(horizontal: false, vertical: true)
                .tabItem { Label("Large Files", systemImage: "externaldrive") }
        }
        .formStyle(.grouped)
        #if os(macOS)
        .frame(width: 460)
        #endif
    }
}
