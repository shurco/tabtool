import SwiftUI
import TabToolKit

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
                Picker("Encoding", selection: bindings.binding(\.encoding)) {
                    ForEach(SupportedEncodings.all) { supported in
                        Text(LocalizedStringKey(supported.name)).tag(supported.encoding)
                    }
                }
                .labelsHidden()
                .accessibilityIdentifier("format-encoding")
                .frame(width: 165)
            }

            control("Separator") {
                Picker("Separator", selection: bindings.binding(\.columnSeparator)) {
                    ForEach(SeparatorOption.allCases) { option in
                        Text(LocalizedStringKey(option.title)).tag(option.rawValue)
                    }
                }
                .labelsHidden()
                .accessibilityIdentifier("format-separator")
                .frame(width: 94)
            }

            control("Quote") {
                Picker("Quote", selection: bindings.quoteStyle) {
                    ForEach(QuoteStyle.allCases) { style in
                        Text(LocalizedStringKey(style.title)).tag(style)
                    }
                }
                .labelsHidden()
                .accessibilityIdentifier("format-quote")
                .frame(width: 102)
            }

            control("Decimal") {
                Picker("Decimal Mark", selection: bindings.binding(\.decimalMark)) {
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
                Toggle("Header", isOn: bindings.binding(\.firstRowAsHeader))
                    .checkboxToggleStyle()
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

    private var bindings: FormatControlBindings {
        FormatControlBindings(configuration: document.configuration,
                              onChange: onConfigurationChange)
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
    static func encodingName(for encoding: String.Encoding, in bundle: Bundle = .main) -> String {
        SupportedEncodings.all.first { $0.encoding == encoding }.map { bundle.localized($0.name) }
            ?? String.localizedName(of: encoding)
    }

    static func separatorName(for separator: String, in bundle: Bundle = .main) -> String {
        SeparatorOption(rawValue: separator).map { bundle.localized($0.name) } ?? separator
    }
}
