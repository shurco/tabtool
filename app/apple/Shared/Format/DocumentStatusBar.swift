import SwiftUI
import TabToolKit

/// A one-line summary of the table size and its format.
///
/// On iOS the line also opens the format sheet: the caller sets
/// `onOpenFormat`, and the whole line becomes a button with a chevron.
struct DocumentStatusBar: View {
    let table: any RowSource
    let configuration: CSVConfiguration
    /// The number of rows a row filter lets through, or `nil` without one.
    var visibleRowCount: Int?
    var status = LargeStatus.none
    var onCancelSave: () -> Void = {}
    /// Opens the format controls; `nil` keeps the line read-only.
    var onOpenFormat: (() -> Void)? = nil

    var body: some View {
        HStack {
            if let onOpenFormat {
                Button(action: onOpenFormat) {
                    HStack {
                        summaryText
                        Spacer()
                        Image(systemName: "chevron.up.chevron.down")
                            .font(.caption)
                            .foregroundStyle(.secondary)
                            .accessibilityHidden(true)
                    }
                    .contentShape(Rectangle())
                }
                .buttonStyle(.plain)
                .accessibilityIdentifier("format-open")
                cancelSaveButton
            } else {
                summaryText
                cancelSaveButton
                Spacer()
            }
        }
        .padding(.horizontal, 12)
        .padding(.vertical, 4)
    }

    @ViewBuilder
    private var cancelSaveButton: some View {
        if case .saving = status {
            Button("Cancel", action: onCancelSave)
                .controlSize(.small)
                .accessibilityIdentifier("cancel-save")
        }
    }

    private var summaryText: some View {
        Text(summary)
            .font(.callout)
            .foregroundStyle(.secondary)
            .accessibilityIdentifier("status-text")
    }

    private var summary: String {
        switch status {
        case .indexing(let fraction):
            return LargeStatusText.indexing(fraction: fraction, rows: table.rowCount)
        case .saving(let fraction):
            return LargeStatusText.saving(fraction: fraction)
        case .none, .ready, .stale:
            break
        }
        let rows = table.rowCount
        let columns = table.columnCount
        let rowsText = Self.rowsText(total: rows, visible: visibleRowCount)
        let columnsText = columns == 1
            ? String(localized: "1 column")
            : String(localized: "\(columns) columns")
        return "\(rowsText) × \(columnsText) · "
            + "\(FormatDisplay.encodingName(for: configuration.encoding)) · "
            + FormatDisplay.separatorName(for: configuration.columnSeparator)
    }

    /// "N rows", or "N of M rows" while a row filter is applied.
    static func rowsText(total: Int, visible: Int?) -> String {
        if let visible {
            return String(localized: "\(visible) of \(total) rows")
        }
        return total == 1 ? String(localized: "1 row") : String(localized: "\(total) rows")
    }
}
