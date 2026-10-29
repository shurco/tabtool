import SwiftUI

/// The About window with the app name, version, and a short description.
struct AboutView: View {
    static let windowID = "about"

    var body: some View {
        VStack(spacing: 14) {
            Image(systemName: "tablecells")
                .font(.system(size: 52, weight: .light))
                .foregroundStyle(.tint)
                .accessibilityHidden(true)

            VStack(spacing: 2) {
                Text("TabTool")
                    .font(.title.weight(.semibold))
                Text("Version \(version)")
                    .font(.callout)
                    .foregroundStyle(.secondary)
            }

            Group {
                #if os(macOS)
                Text("A simple CSV editor for macOS.")
                #else
                Text("A simple CSV editor for iPhone and iPad.")
                #endif
            }
            .multilineTextAlignment(.center)
            .frame(maxWidth: 360)
        }
        .padding(24)
        #if os(macOS)
        .frame(width: 420)
        #else
        .frame(maxWidth: 420)
        #endif
    }

    private var version: String {
        Bundle.main.object(forInfoDictionaryKey: "CFBundleShortVersionString") as? String
            ?? "1.0"
    }
}
