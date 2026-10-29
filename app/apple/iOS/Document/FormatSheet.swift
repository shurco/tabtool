import SwiftUI
import TabToolKit

/// The format controls on iOS, presented from the status line.
///
/// Every control reports a complete candidate configuration; the document
/// view derives the operation with `FormatChangePlanner`.
struct FormatSheet: View {
    @ObservedObject var document: CSVDocument
    let isBusy: Bool
    let isDetectAvailable: Bool
    let onConfigurationChange: (CSVConfiguration) -> Void
    let onDetectAutomatically: () -> Void

    @Environment(\.dismiss) private var dismiss

    var body: some View {
        NavigationStack {
            Form {
                Picker("Encoding", selection: bindings.binding(\.encoding)) {
                    ForEach(SupportedEncodings.all) { supported in
                        Text(LocalizedStringKey(supported.name)).tag(supported.encoding)
                    }
                }
                .accessibilityIdentifier("format-encoding")

                Picker("Separator", selection: bindings.binding(\.columnSeparator)) {
                    ForEach(SeparatorOption.allCases) { option in
                        Text(LocalizedStringKey(option.title)).tag(option.rawValue)
                    }
                }
                .accessibilityIdentifier("format-separator")

                Picker("Quote", selection: bindings.quoteStyle) {
                    ForEach(QuoteStyle.allCases) { style in
                        Text(LocalizedStringKey(style.title)).tag(style)
                    }
                }
                .accessibilityIdentifier("format-quote")

                Picker("Decimal Mark", selection: bindings.binding(\.decimalMark)) {
                    ForEach(DecimalOption.allCases) { option in
                        Text(LocalizedStringKey(option.title)).tag(option.rawValue)
                    }
                }
                .accessibilityIdentifier("format-decimal")
                .disabled(document.isLargeMode)

                Toggle("First Row as Header", isOn: bindings.binding(\.firstRowAsHeader))
                    .accessibilityIdentifier("format-header")

                Button("Detect Automatically", action: onDetectAutomatically)
                    .accessibilityIdentifier("format-detect")
                    .disabled(isBusy || !isDetectAvailable)

                Section {
                } footer: {
                    Text("Changing the format re-reads the file.")
                }
            }
            .disabled(isBusy)
            .navigationTitle("Format")
            .toolbar {
                ToolbarItem(placement: .confirmationAction) {
                    Button("Done") { dismiss() }
                }
            }
        }
        .presentationDetents([.medium, .large])
    }

    private var bindings: FormatControlBindings {
        FormatControlBindings(configuration: document.configuration,
                              onChange: onConfigurationChange)
    }
}
