import SwiftUI

/// Explains what large file mode allows. Shown for the whole life of the window.
struct LargeFileBanner: View {
    var body: some View {
        HStack(alignment: .center, spacing: 10) {
            Image(systemName: "info.circle.fill")
                .foregroundStyle(.blue)
                .accessibilityHidden(true)
            bannerText
                .font(.callout)
                .foregroundStyle(.secondary)
            Spacer(minLength: 12)
        }
        .padding(.horizontal, 12)
        .padding(.vertical, 8)
        .background(Color.blue.opacity(0.10))
        .accessibilityElement(children: .combine)
        .accessibilityIdentifier("large-file-banner")
    }

    private var bannerText: Text {
        Text("Large file mode. You can browse and edit cells. Sorting, adding, deleting or moving rows and columns, renaming columns, changing the decimal mark and exporting are not available for files this size.")
    }
}

/// Tells the user the file changed on disk and offers to read it again.
struct LargeFileStaleBanner: View {
    let appended: Bool
    let onReload: () -> Void

    var body: some View {
        layout
            .padding(.horizontal, 12)
            .padding(.vertical, 8)
            .background(Color.yellow.opacity(0.14))
            .accessibilityElement(children: .contain)
            .accessibilityIdentifier("large-file-stale-banner")
    }

    /// The window-shaped strip on macOS; a stack on a phone, where the text
    /// and the Reload button do not fit side by side.
    private var layout: some View {
        #if os(iOS)
        VStack(alignment: .leading, spacing: 8) {
            HStack(alignment: .firstTextBaseline, spacing: 10) {
                warningIcon
                message
            }
            HStack {
                Spacer(minLength: 0)
                reloadButton
                    .controlSize(.regular)
            }
        }
        #else
        HStack(alignment: .center, spacing: 10) {
            warningIcon
            message
            Spacer(minLength: 12)
            reloadButton
                .controlSize(.small)
        }
        #endif
    }

    private var warningIcon: some View {
        Image(systemName: "exclamationmark.triangle.fill")
            .foregroundStyle(.yellow)
            .accessibilityHidden(true)
    }

    private var message: some View {
        Text(appended ? "New rows were appended. Reload to see them."
                      : "The file changed on disk. Reload to continue.")
            .font(.callout.weight(.semibold))
    }

    private var reloadButton: some View {
        Button("Reload", action: onReload)
            .accessibilityIdentifier("large-file-reload")
    }
}
