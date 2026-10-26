import SwiftUI

/// The About window with the app name, version, and a short description.
struct AboutView: View {
    static let windowID = "about"
    static let repositoryURL = URL(string: "https://example.com")!
    static let licenseURL = URL(string: "https://example.com/LICENSE")!

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

            Text("A simple CSV editor for macOS. This is a free, MIT-licensed "
                + "independent CSV editor.")
                .multilineTextAlignment(.center)
                .frame(maxWidth: 360)

            Divider()

            VStack(spacing: 4) {
                Text("About This App")
                    .font(.headline)
                Text("Dmitry Shurco")
                    .font(.callout)
                Link("Project Repository", destination: Self.repositoryURL)
                    .font(.callout)
                Link("MIT License", destination: Self.licenseURL)
                    .font(.callout)
            }

            Text("MIT License. Copyright © 2016 Dmitry Shurco, "
                + "© 2015–2016 Dmitry Shurco, © 2015 Dmitry Shurco.")
                .font(.caption)
                .foregroundStyle(.secondary)
                .multilineTextAlignment(.center)
                .frame(maxWidth: 380)
        }
        .padding(24)
        .frame(width: 420)
    }

    private var version: String {
        Bundle.main.object(forInfoDictionaryKey: "CFBundleShortVersionString") as? String
            ?? "1.0"
    }
}
