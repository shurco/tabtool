import SwiftUI
import TabToolKit

/// The export format edited in the save panel's accessory view.
///
/// The document configuration seeds the model; every change stays local to
/// the export and never touches the document.
@MainActor
final class ExportOptions: ObservableObject {
    @Published var format: ExportFormat = .csv
    @Published var configuration: CSVConfiguration

    init(configuration: CSVConfiguration) {
        self.configuration = configuration
    }
}

/// Format, encoding, separator, quote, decimal, and header controls for an
/// export. JSON and XML use only the decimal control: they are always UTF-8
/// and take their keys from the column names.
///
/// Hosted by `ExportCoordinator` inside `NSSavePanel.accessoryView`. The
/// controls call `onChange` after every edit so the coordinator can keep the
/// panel's file type in sync.
struct ExportOptionsView: View {
    @ObservedObject var options: ExportOptions
    let onChange: () -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            Grid(alignment: .leadingFirstTextBaseline,
                 horizontalSpacing: 12, verticalSpacing: 8) {
                row("Format") {
                    Picker("Format", selection: formatBinding) {
                        ForEach(ExportFormat.allCases) { format in
                            Text(verbatim: format.rawValue.uppercased()).tag(format)
                        }
                    }
                    .labelsHidden()
                    .frame(width: 190, alignment: .leading)
                    .accessibilityIdentifier("export-format")
                }

                row("Encoding", csvOnly: true) {
                    Picker("Encoding", selection: binding(\.encoding)) {
                        ForEach(SupportedEncodings.all) { supported in
                            Text(LocalizedStringKey(supported.name)).tag(supported.encoding)
                        }
                    }
                    .labelsHidden()
                    .frame(width: 190, alignment: .leading)
                    .accessibilityIdentifier("export-encoding")
                }

                row("Separator", csvOnly: true) {
                    Picker("Separator", selection: binding(\.columnSeparator)) {
                        ForEach(SeparatorOption.allCases) { option in
                            Text(LocalizedStringKey(option.title)).tag(option.rawValue)
                        }
                    }
                    .labelsHidden()
                    .frame(width: 190, alignment: .leading)
                    .accessibilityIdentifier("export-separator")
                }

                row("Quote", csvOnly: true) {
                    Picker("Quote", selection: quoteStyleBinding) {
                        ForEach(QuoteStyle.allCases) { style in
                            Text(LocalizedStringKey(style.title)).tag(style)
                        }
                    }
                    .labelsHidden()
                    .frame(width: 190, alignment: .leading)
                    .accessibilityIdentifier("export-quote")
                }

                row("Decimal") {
                    Picker("Decimal Mark", selection: binding(\.decimalMark)) {
                        ForEach(DecimalOption.allCases) { option in
                            Text(LocalizedStringKey(option.title)).tag(option.rawValue)
                        }
                    }
                    .labelsHidden()
                    .frame(width: 190, alignment: .leading)
                    .accessibilityIdentifier("export-decimal")
                }

                row("Header", csvOnly: true) {
                    Toggle("Write Header Row", isOn: binding(\.firstRowAsHeader))
                        .checkboxToggleStyle()
                        .labelsHidden()
                        .accessibilityLabel("Write Header Row")
                        .accessibilityIdentifier("export-header")
                }
            }

            Group {
                if options.format == .csv {
                    Text("The header is written only to the exported file.")
                } else {
                    Text("Column names become keys. The file is written in UTF-8.")
                }
            }
            .font(.caption)
            .foregroundStyle(.secondary)
        }
        .controlSize(.small)
        .padding(16)
        #if os(macOS)
        .frame(width: 340, alignment: .leading)
        #else
        .frame(maxWidth: .infinity, alignment: .leading)
        #endif
    }

    /// A labeled grid row. A `csvOnly` row is disabled for JSON and XML; it
    /// stays visible so the save panel's accessory view keeps its size.
    private func row<Content: View>(
        _ title: String,
        csvOnly: Bool = false,
        @ViewBuilder content: () -> Content
    ) -> some View {
        let disabled = csvOnly && options.format != .csv
        return GridRow {
            Text(LocalizedStringKey(title))
                .foregroundStyle(disabled ? .secondary : .primary)
            content()
                .disabled(disabled)
        }
    }

    private func binding<Value>(
        _ keyPath: WritableKeyPath<CSVConfiguration, Value>
    ) -> Binding<Value> {
        Binding(
            get: { options.configuration[keyPath: keyPath] },
            set: { newValue in
                options.configuration[keyPath: keyPath] = newValue
                onChange()
            }
        )
    }

    private var quoteStyleBinding: Binding<QuoteStyle> {
        Binding(
            get: { QuoteStyle(configuration: options.configuration) },
            set: { style in
                options.configuration.quoteCharacter = style.quoteCharacter
                options.configuration.escapeCharacter = style.escapeCharacter
                onChange()
            }
        )
    }

    private var formatBinding: Binding<ExportFormat> {
        Binding(
            get: { options.format },
            set: { format in
                options.format = format
                onChange()
            }
        )
    }
}
