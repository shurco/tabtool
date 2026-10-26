import SwiftUI
import TabToolKit

/// Explains why the document cannot be saved and offers a way out.
///
/// Shown above the format bar whenever the current table would produce
/// unrepresentable or ambiguous output. The document is still editable; the
/// banner is a persistent warning, not a modal error.
struct NeedsAttentionBanner: View {
    let issue: CSVWriteIssue
    let canUndo: Bool
    let onUndo: () -> Void
    let onShowFormatControls: () -> Void

    var body: some View {
        HStack(alignment: .center, spacing: 10) {
            Image(systemName: "exclamationmark.triangle.fill")
                .foregroundStyle(.yellow)
                .accessibilityHidden(true)

            VStack(alignment: .leading, spacing: 2) {
                Text(title)
                    .font(.callout.weight(.semibold))
                Text(detail)
                    .font(.callout)
                    .foregroundStyle(.secondary)
            }

            Spacer(minLength: 12)

            Button("Undo Last Edit", action: onUndo)
                .disabled(!canUndo)
                .accessibilityIdentifier("undo-last-edit")

            Button("Show Format Controls", action: onShowFormatControls)
                .accessibilityIdentifier("show-format-controls")
        }
        .controlSize(.small)
        .padding(.horizontal, 12)
        .padding(.vertical, 8)
        .background(Color.yellow.opacity(0.14))
        .accessibilityElement(children: .contain)
        .accessibilityIdentifier("needs-attention-banner")
    }

    private var title: String {
        switch issue {
        case .unencodableText:
            return "This document can’t be saved in its current encoding."
        case .ambiguousOutput:
            return "This document can’t be saved with quotes off."
        }
    }

    private var detail: String {
        switch issue {
        case .unencodableText:
            return "Some text does not exist in the selected encoding. "
                + "Choose another encoding, or undo the edit that added it."
        case .ambiguousOutput:
            return "A cell contains the separator or a line break. "
                + "Turn quotes on, or undo the edit that added it."
        }
    }
}
