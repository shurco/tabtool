import SwiftUI
import TabToolKit

/// The user default shared by the View menu and the document window.
enum FormatBarVisibility {
    static let storageKey = "showFormatBar"
}

/// The format controls at the bottom of the document window.
///
/// The bar reports a complete candidate configuration for every edit; the
/// document view derives the operation with `FormatChangePlanner`, so this
/// view never changes the document directly.
struct FormatBar: View {
    @ObservedObject var document: CSVDocument
    let isBusy: Bool
    let isDetectAvailable: Bool
    let onConfigurationChange: (CSVConfiguration) -> Void
    let onDetectAutomatically: () -> Void

    var body: some View {
        HStack(alignment: .bottom, spacing: 10) {
            control("Encoding") {
                Picker("Encoding", selection: binding(\.encoding)) {
                    ForEach(SupportedEncodings.all) { supported in
                        Text(LocalizedStringKey(supported.name)).tag(supported.encoding)
                    }
                }
                .labelsHidden()
                .accessibilityIdentifier("format-encoding")
                .frame(width: 165)
            }

            control("Separator") {
                Picker("Separator", selection: binding(\.columnSeparator)) {
                    ForEach(SeparatorOption.allCases) { option in
                        Text(LocalizedStringKey(option.title)).tag(option.rawValue)
                    }
                }
                .labelsHidden()
                .accessibilityIdentifier("format-separator")
                .frame(width: 94)
            }

            control("Quote") {
                Picker("Quote", selection: quoteStyleBinding) {
                    ForEach(QuoteStyle.allCases) { style in
                        Text(LocalizedStringKey(style.title)).tag(style)
                    }
                }
                .labelsHidden()
                .accessibilityIdentifier("format-quote")
                .frame(width: 102)
            }

            control("Decimal") {
                Picker("Decimal Mark", selection: binding(\.decimalMark)) {
                    ForEach(DecimalOption.allCases) { option in
                        Text(LocalizedStringKey(option.title)).tag(option.rawValue)
                    }
                }
                .labelsHidden()
                .accessibilityIdentifier("format-decimal")
                .frame(width: 84)
                .disabled(document.isLargeMode)
                .help(document.isLargeMode ? Text("Not available in large file mode.") : Text(verbatim: ""))
            }

            control("Header") {
                Toggle("Header", isOn: binding(\.firstRowAsHeader))
                    .toggleStyle(.checkbox)
                    .labelsHidden()
                    .accessibilityLabel("Header")
                    .accessibilityIdentifier("format-header")
            }

            Spacer(minLength: 0)

            Button(action: onDetectAutomatically) {
                Label("Detect Automatically", systemImage: "wand.and.stars")
                    .labelStyle(.iconOnly)
            }
            .help("Detect Format Automatically")
            .accessibilityLabel("Detect Format Automatically")
            .accessibilityIdentifier("format-detect")
            .disabled(!isDetectAvailable)
            .padding(.bottom, 1)
        }
        .controlSize(.small)
        .padding(.horizontal, 10)
        .padding(.vertical, 6)
        .disabled(isBusy)
    }

    /// A compact caption above a control, keeping the bar usable at the
    /// window's minimum width.
    private func control<Content: View>(
        _ title: String,
        @ViewBuilder content: () -> Content
    ) -> some View {
        VStack(alignment: .leading, spacing: 2) {
            Text(LocalizedStringKey(title))
                .font(.caption2)
                .foregroundStyle(.secondary)
                .accessibilityHidden(true)
            content()
        }
    }

    private func binding<Value>(
        _ keyPath: WritableKeyPath<CSVConfiguration, Value>
    ) -> Binding<Value> {
        Binding(
            get: { document.configuration[keyPath: keyPath] },
            set: { newValue in
                var configuration = document.configuration
                configuration[keyPath: keyPath] = newValue
                onConfigurationChange(configuration)
            }
        )
    }

    private var quoteStyleBinding: Binding<QuoteStyle> {
        Binding(
            get: { QuoteStyle(configuration: document.configuration) },
            set: { style in
                var configuration = document.configuration
                configuration.quoteCharacter = style.quoteCharacter
                configuration.escapeCharacter = style.escapeCharacter
                onConfigurationChange(configuration)
            }
        )
    }
}

/// A one-line summary of the table size and its format.
struct DocumentStatusBar: View {
    let table: any RowSource
    let configuration: CSVConfiguration
    var status = LargeStatus.none
    var onCancelSave: () -> Void = {}

    var body: some View {
        HStack {
            Text(summary)
                .font(.callout)
                .foregroundStyle(.secondary)
                .accessibilityIdentifier("status-text")
            if case .saving = status {
                Button("Cancel", action: onCancelSave)
                    .controlSize(.small)
                    .accessibilityIdentifier("cancel-save")
            }
            Spacer()
        }
        .padding(.horizontal, 12)
        .padding(.vertical, 4)
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
        let rowsText = rows == 1
            ? String(localized: "1 row")
            : String(localized: "\(rows) rows")
        let columnsText = columns == 1
            ? String(localized: "1 column")
            : String(localized: "\(columns) columns")
        return "\(rowsText) × \(columnsText) · "
            + "\(FormatDisplay.encodingName(for: configuration.encoding)) · "
            + FormatDisplay.separatorName(for: configuration.columnSeparator)
    }
}

/// The quote styles offered by the format bar and the export panel.
enum QuoteStyle: String, CaseIterable, Identifiable {
    case doubled
    case backslash
    case none

    var id: String { rawValue }

    var title: String {
        switch self {
        case .doubled: return "Doubled"
        case .backslash: return "Backslash"
        case .none: return "None"
        }
    }

    var quoteCharacter: String {
        self == .none ? "" : "\""
    }

    var escapeCharacter: String {
        switch self {
        case .doubled: return "\""
        case .backslash: return "\\"
        case .none: return ""
        }
    }

    init(configuration: CSVConfiguration) {
        if configuration.quoteCharacter.isEmpty {
            self = .none
        } else if configuration.escapeCharacter == "\\" {
            self = .backslash
        } else {
            self = .doubled
        }
    }
}

/// The separators offered by the format bar and the export panel.
enum SeparatorOption: String, CaseIterable, Identifiable {
    case comma = ","
    case semicolon = ";"
    case tab = "\t"
    case pipe = "|"

    var id: String { rawValue }

    var name: String {
        switch self {
        case .comma: return "Comma"
        case .semicolon: return "Semicolon"
        case .tab: return "Tab"
        case .pipe: return "Pipe"
        }
    }

    var title: String {
        let symbol = self == .tab ? "⇥" : rawValue
        return "\(name) (\(symbol))"
    }
}

/// The decimal marks offered by the format bar and the export panel.
enum DecimalOption: String, CaseIterable, Identifiable {
    case dot = "."
    case comma = ","

    var id: String { rawValue }

    var title: String {
        switch self {
        case .dot: return "Dot (.)"
        case .comma: return "Comma (,)"
        }
    }
}

/// Human-readable names shared by the format bar and the status line.
enum FormatDisplay {
    static func encodingName(for encoding: String.Encoding) -> String {
        SupportedEncodings.all.first { $0.encoding == encoding }?.name
            ?? String.localizedName(of: encoding)
    }

    static func separatorName(for separator: String) -> String {
        SeparatorOption(rawValue: separator)?.name ?? separator
    }
}
