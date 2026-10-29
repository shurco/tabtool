import SwiftUI

/// The About window with the app name, version, a short description, the
/// website, and the license.
struct AboutView: View {
    static let windowID = "about"
    static let websiteURL = URL(string: "https://tabtool.shurco.app")!

    var body: some View {
        VStack(spacing: 14) {
            Image("AppMark")
                .resizable()
                .frame(width: 72, height: 72)
                .accessibilityHidden(true)

            VStack(spacing: 2) {
                Text("TabTool")
                    .font(.title.weight(.semibold))
                Text("Version \(version)")
                    .font(.callout)
                    .foregroundStyle(.secondary)
            }

            VStack(spacing: 10) {
                Group {
                    #if os(macOS)
                    Text("A simple CSV editor for macOS.")
                    #else
                    Text("A simple CSV editor for iPhone and iPad.")
                    #endif
                }
                Text("Opens, edits, and saves CSV, TSV, and plain text tables. Detects the encoding and separator, keeps large files responsive, filters and sorts rows, and exports in the format you choose.")
            }
            .multilineTextAlignment(.center)
            .frame(maxWidth: 360)

            Link(destination: Self.websiteURL) {
                Text(verbatim: "tabtool.shurco.app")
                    .font(.callout)
            }
            .accessibilityIdentifier("about-website")

            Divider()
                .frame(maxWidth: 320)

            VStack(spacing: 4) {
                Text("MIT License")
                    .font(.footnote.weight(.semibold))
                Text("Free and open source. Copyright © 2026 Dmitry Shurco.")
                    .font(.footnote)
                    .foregroundStyle(.secondary)
            }
            .multilineTextAlignment(.center)
            .accessibilityIdentifier("about-license")
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
