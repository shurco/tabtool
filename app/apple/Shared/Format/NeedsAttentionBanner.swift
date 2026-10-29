import Accessibility
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
        layout
            .controlSize(controlSize)
            .padding(.horizontal, 12)
            .padding(.vertical, 8)
            .background(Color.yellow.opacity(0.14))
            .accessibilityElement(children: .contain)
            .accessibilityIdentifier("needs-attention-banner")
            .onAppear {
                AccessibilityNotification.Announcement("\(title) \(detail)").post()
            }
    }

    /// The window-shaped strip on macOS; a stack on a phone, where the two
    /// buttons and the text do not fit side by side.
    private var layout: some View {
        #if os(iOS)
        VStack(alignment: .leading, spacing: 8) {
            HStack(alignment: .firstTextBaseline, spacing: 10) {
                warningIcon
                titleText
            }
            detailText
            HStack(spacing: 12) {
                Spacer(minLength: 0)
                Button(action: onUndo) {
                    Text("Undo Last Edit")
                        .lineLimit(nil)
                        .multilineTextAlignment(.center)
                }
                .disabled(!canUndo)
                .accessibilityIdentifier("undo-last-edit")
                Button(action: onShowFormatControls) {
                    Text("Show Format Controls")
                        .lineLimit(nil)
                        .multilineTextAlignment(.center)
                }
                .accessibilityIdentifier("show-format-controls")
            }
        }
        #else
        HStack(alignment: .center, spacing: 10) {
            warningIcon
            VStack(alignment: .leading, spacing: 2) {
                titleText
                detailText
            }
            Spacer(minLength: 12)
            Button("Undo Last Edit", action: onUndo)
                .disabled(!canUndo)
                .accessibilityIdentifier("undo-last-edit")
            Button("Show Format Controls", action: onShowFormatControls)
                .accessibilityIdentifier("show-format-controls")
        }
        #endif
    }

    private var controlSize: ControlSize {
        #if os(iOS)
        .regular
        #else
        .small
        #endif
    }

    private var warningIcon: some View {
        Image(systemName: "exclamationmark.triangle.fill")
            .foregroundStyle(.yellow)
            .accessibilityHidden(true)
    }

    private var titleText: some View {
        Text(verbatim: title)
            .font(.callout.weight(.semibold))
    }

    private var detailText: some View {
        Text(verbatim: detail)
            .font(.callout)
            .foregroundStyle(.secondary)
    }

    private var title: String {
        switch issue {
        case .unencodableText:
            return String(localized: "This document can’t be saved in its current encoding.")
        case .ambiguousOutput:
            return String(localized: "This document can’t be saved with quotes off.")
        }
    }

    private var detail: String {
        switch issue {
        case .unencodableText:
            return String(localized: "Some text does not exist in the selected encoding. Undo the edit that added it, or export the table in an encoding that can represent it.")
        case .ambiguousOutput:
            return String(localized: "A cell contains the separator or a line break and quotes are off. Undo the edit that added it, or export with quotes on.")
        }
    }
}
