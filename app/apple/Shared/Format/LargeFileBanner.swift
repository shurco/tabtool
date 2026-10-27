import SwiftUI

/// Explains what large file mode allows. Shown for the whole life of the window.
struct LargeFileBanner: View {
    var body: some View {
        HStack(alignment: .center, spacing: 10) {
            Image(systemName: "info.circle.fill")
                .foregroundStyle(.blue)
                .accessibilityHidden(true)
            Text("Large file mode. You can browse and edit cells. Sorting, adding, deleting or moving rows and columns, renaming columns, changing the decimal mark and exporting are not available for files this size.")
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
}

/// Tells the user the file changed on disk and offers to read it again.
struct LargeFileStaleBanner: View {
    let appended: Bool
    let onReload: () -> Void

    var body: some View {
        HStack(alignment: .center, spacing: 10) {
            Image(systemName: "exclamationmark.triangle.fill")
                .foregroundStyle(.yellow)
                .accessibilityHidden(true)
            Text(appended ? "New rows were appended. Reload to see them."
                          : "The file changed on disk. Reload to continue.")
                .font(.callout.weight(.semibold))
            Spacer(minLength: 12)
            Button("Reload", action: onReload)
                .controlSize(.small)
                .accessibilityIdentifier("large-file-reload")
        }
        .padding(.horizontal, 12)
        .padding(.vertical, 8)
        .background(Color.yellow.opacity(0.14))
        .accessibilityElement(children: .contain)
        .accessibilityIdentifier("large-file-stale-banner")
    }
}
