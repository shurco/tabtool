import SwiftUI
import TabToolKit

/// The export format edited in the save panel's accessory view.
///
/// The document configuration seeds the model; every change stays local to
/// the export and never touches the document.
@MainActor
final class ExportOptions: ObservableObject {
    @Published var configuration: CSVConfiguration

    init(configuration: CSVConfiguration) {
        self.configuration = configuration
    }
}

/// Encoding, separator, quote, decimal, and header controls for an export.
///
/// Hosted by `ExportCoordinator` inside `NSSavePanel.accessoryView`. The
/// controls report the complete candidate configuration after every edit so
/// the coordinator can keep the panel's file type in sync.
struct ExportOptionsView: View {
    @ObservedObject var options: ExportOptions
    let onConfigurationChange: (CSVConfiguration) -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            Grid(alignment: .leadingFirstTextBaseline,
                 horizontalSpacing: 12, verticalSpacing: 8) {
                row("Encoding") {
                    Picker("Encoding", selection: binding(\.encoding)) {
                        ForEach(SupportedEncodings.all) { supported in
                            Text(supported.name).tag(supported.encoding)
                        }
                    }
                    .labelsHidden()
                    .frame(width: 190, alignment: .leading)
                    .accessibilityIdentifier("export-encoding")
                }

                row("Separator") {
                    Picker("Separator", selection: binding(\.columnSeparator)) {
                        ForEach(SeparatorOption.allCases) { option in
                            Text(option.title).tag(option.rawValue)
                        }
                    }
                    .labelsHidden()
                    .frame(width: 190, alignment: .leading)
                    .accessibilityIdentifier("export-separator")
                }

                row("Quote") {
                    Picker("Quote", selection: quoteStyleBinding) {
                        ForEach(QuoteStyle.allCases) { style in
                            Text(style.title).tag(style)
                        }
                    }
                    .labelsHidden()
                    .frame(width: 190, alignment: .leading)
                    .accessibilityIdentifier("export-quote")
                }

                row("Decimal") {
                    Picker("Decimal Mark", selection: binding(\.decimalMark)) {
                        ForEach(DecimalOption.allCases) { option in
                            Text(option.title).tag(option.rawValue)
                        }
                    }
                    .labelsHidden()
                    .frame(width: 190, alignment: .leading)
                    .accessibilityIdentifier("export-decimal")
                }

                row("Header") {
                    Toggle("Write Header Row", isOn: binding(\.firstRowAsHeader))
                        .toggleStyle(.checkbox)
                        .labelsHidden()
                        .accessibilityLabel("Write Header Row")
                        .accessibilityIdentifier("export-header")
                }
            }

            Text("The header is written only to the exported file.")
                .font(.caption)
                .foregroundStyle(.secondary)
        }
        .controlSize(.small)
        .padding(16)
        .frame(width: 340, alignment: .leading)
    }

    private func row<Content: View>(
        _ title: String,
        @ViewBuilder content: () -> Content
    ) -> some View {
        GridRow {
            Text(title)
            content()
        }
    }

    private func binding<Value>(
        _ keyPath: WritableKeyPath<CSVConfiguration, Value>
    ) -> Binding<Value> {
        Binding(
            get: { options.configuration[keyPath: keyPath] },
            set: { newValue in
                options.configuration[keyPath: keyPath] = newValue
                onConfigurationChange(options.configuration)
            }
        )
    }

    private var quoteStyleBinding: Binding<QuoteStyle> {
        Binding(
            get: { QuoteStyle(configuration: options.configuration) },
            set: { style in
                options.configuration.quoteCharacter = style.quoteCharacter
                options.configuration.escapeCharacter = style.escapeCharacter
                onConfigurationChange(options.configuration)
            }
        )
    }
}
