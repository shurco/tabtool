import SwiftUI
import TabToolKit

/// The Settings window.
///
/// Every control writes straight to the stored preference; open documents
/// react to the grid and format bar preferences through their own `@AppStorage`.
struct SettingsView: View {
    @AppStorage(Preferences.Key.appLanguage) private var language = Preferences.Default.appLanguage
    @AppStorage(Preferences.Key.showFormatBar) private var showFormatBar = Preferences.Default.showFormatBar
    @AppStorage(Preferences.Key.newDocumentEncoding) private var encodingID = Preferences.Default.encodingID
    @AppStorage(Preferences.Key.newDocumentSeparator) private var separator = Preferences.Default.separator
    @AppStorage(Preferences.Key.newDocumentQuoteStyle) private var quoteStyle = Preferences.Default.quoteStyle
    @AppStorage(Preferences.Key.newDocumentDecimalMark) private var decimalMark = Preferences.Default.decimalMark
    @AppStorage(Preferences.Key.newDocumentFirstRowAsHeader)
    private var firstRowAsHeader = Preferences.Default.firstRowAsHeader
    @AppStorage(Preferences.Key.gridAlternatingRows) private var alternatingRows = Preferences.Default.alternatingRows
    @AppStorage(Preferences.Key.gridRowHeight) private var rowHeight = Preferences.Default.rowHeight
    @AppStorage(Preferences.Key.largeFileThresholdMiB)
    private var largeFileThresholdMiB = Preferences.Default.largeFileThresholdMiB

    var body: some View {
        Form {
            Section("General") {
                Picker("Language", selection: $language) {
                    ForEach(AppLanguage.allCases) { option in
                        if let name = option.nativeName {
                            Text(verbatim: name).tag(option)
                        } else {
                            Text("System").tag(option)
                        }
                    }
                }
                .accessibilityIdentifier("settings-language")
                .onChange(of: language) { _, new in new.applyOverride() }
                caption("Takes effect after restart.")

                Toggle("Show Format Bar", isOn: $showFormatBar)
                    .accessibilityIdentifier("settings-show-format-bar")
            }

            Section("New Document") {
                Picker("Encoding", selection: $encodingID) {
                    ForEach(SupportedEncodings.all) { supported in
                        Text(LocalizedStringKey(supported.name)).tag(Int(supported.id))
                    }
                }
                .accessibilityIdentifier("settings-encoding")

                Picker("Separator", selection: $separator) {
                    ForEach(SeparatorOption.allCases) { option in
                        Text(LocalizedStringKey(option.title)).tag(option)
                    }
                }
                .accessibilityIdentifier("settings-separator")

                Picker("Quote", selection: $quoteStyle) {
                    ForEach(QuoteStyle.allCases) { style in
                        Text(LocalizedStringKey(style.title)).tag(style)
                    }
                }
                .accessibilityIdentifier("settings-quote")

                Picker("Decimal Mark", selection: $decimalMark) {
                    ForEach(DecimalOption.allCases) { option in
                        Text(LocalizedStringKey(option.title)).tag(option)
                    }
                }
                .accessibilityIdentifier("settings-decimal")

                Toggle("First Row as Header", isOn: $firstRowAsHeader)
                    .accessibilityIdentifier("settings-header")
                caption("These apply to new documents. Opened files keep the format detected from their content.")
            }

            Section("Grid") {
                Toggle("Alternating Row Colors", isOn: $alternatingRows)
                    .accessibilityIdentifier("settings-alternating-rows")

                Picker("Row Height", selection: $rowHeight) {
                    ForEach(Preferences.rowHeights, id: \.self) { height in
                        Text(rowHeightTitle(height)).tag(height)
                    }
                }
                .accessibilityIdentifier("settings-row-height")
            }

            Section("Large Files") {
                Picker("Large File Threshold", selection: $largeFileThresholdMiB) {
                    ForEach(Preferences.largeFileThresholdsMiB, id: \.self) { mebibytes in
                        Text(Int64(mebibytes) * 1_048_576, format: .byteCount(style: .binary)).tag(mebibytes)
                    }
                }
                .accessibilityIdentifier("settings-large-threshold")
                caption("Files at or above this size open in large file mode.")
            }
        }
        .formStyle(.grouped)
        .frame(width: 460)
        .fixedSize(horizontal: false, vertical: true)
    }

    private func caption(_ text: LocalizedStringKey) -> some View {
        Text(text)
            .font(.caption)
            .foregroundStyle(.secondary)
    }

    private func rowHeightTitle(_ height: Double) -> LocalizedStringKey {
        switch height {
        case ..<Preferences.Default.rowHeight: "Compact"
        case Preferences.Default.rowHeight: "Default"
        default: "Comfortable"
        }
    }
}
