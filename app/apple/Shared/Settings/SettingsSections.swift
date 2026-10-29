import SwiftUI
import TabToolKit

/// The rows of the General settings section.
struct GeneralSettings: View {
    @AppStorage(Preferences.Key.appLanguage) private var language = Preferences.Default.appLanguage
    #if os(macOS)
    @AppStorage(Preferences.Key.showFormatBar) private var showFormatBar = Preferences.Default.showFormatBar
    #endif
    @State private var needsRestart = false

    var body: some View {
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
        .onChange(of: language) { _, new in
            new.applyOverride()
            needsRestart = true
        }
        #if os(macOS)
        HStack {
            caption("Takes effect after restart.")
            if needsRestart {
                Spacer()
                Button { AppRelaunch.request() } label: {
                    Label("Restart Now", systemImage: "arrow.clockwise")
                }
                .labelStyle(.iconOnly)
                .buttonStyle(.borderless)
                .help("Restart Now")
                .accessibilityIdentifier("settings-restart")
            }
        }
        #else
        if needsRestart {
            caption("Open TabTool again to apply it.")
        }
        #endif

        #if os(macOS)
        Toggle("Show Format Bar", isOn: $showFormatBar)
            .accessibilityIdentifier("settings-show-format-bar")
        #endif
    }
}

/// The rows of the New Document settings section.
struct NewDocumentSettings: View {
    @AppStorage(Preferences.Key.newDocumentEncoding) private var encodingID = Preferences.Default.encodingID
    @AppStorage(Preferences.Key.newDocumentSeparator) private var separator = Preferences.Default.separator
    @AppStorage(Preferences.Key.newDocumentQuoteStyle) private var quoteStyle = Preferences.Default.quoteStyle
    @AppStorage(Preferences.Key.newDocumentDecimalMark) private var decimalMark = Preferences.Default.decimalMark
    @AppStorage(Preferences.Key.newDocumentFirstRowAsHeader)
    private var firstRowAsHeader = Preferences.Default.firstRowAsHeader

    var body: some View {
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
}

/// The rows of the Grid settings section.
struct GridSettings: View {
    @AppStorage(Preferences.Key.gridAlternatingRows) private var alternatingRows = Preferences.Default.alternatingRows
    @AppStorage(Preferences.Key.gridRowHeight) private var rowHeight = Preferences.Default.rowHeight
    @AppStorage(Preferences.Key.gridShowsRowNumbers) private var showsRowNumbers = Preferences.Default.showsRowNumbers

    var body: some View {
        Toggle("Alternating Row Colors", isOn: $alternatingRows)
            .accessibilityIdentifier("settings-alternating-rows")

        Toggle("Show Row Numbers", isOn: $showsRowNumbers)
            .accessibilityIdentifier("settings-row-numbers")

        Picker("Row Height", selection: $rowHeight) {
            ForEach(Preferences.rowHeights, id: \.self) { height in
                Text(rowHeightTitle(height)).tag(height)
            }
        }
        .accessibilityIdentifier("settings-row-height")
    }

    private func rowHeightTitle(_ height: Double) -> LocalizedStringKey {
        switch height {
        case ..<Preferences.Default.rowHeight: "Compact"
        case Preferences.Default.rowHeight: "Default"
        default: "Comfortable"
        }
    }
}

/// The rows of the Large Files settings section.
struct LargeFileSettings: View {
    @AppStorage(Preferences.Key.largeFileThresholdMiB)
    private var largeFileThresholdMiB = Preferences.Default.largeFileThresholdMiB

    var body: some View {
        Picker("Large File Threshold", selection: $largeFileThresholdMiB) {
            ForEach(Preferences.largeFileThresholdsMiB, id: \.self) { mebibytes in
                Text(Int64(mebibytes) * 1_048_576, format: .byteCount(style: .binary)).tag(mebibytes)
            }
        }
        .accessibilityIdentifier("settings-large-threshold")
        caption("Files at or above this size open in large file mode.")
    }
}

private func caption(_ text: LocalizedStringKey) -> some View {
    Text(text)
        .font(.caption)
        .foregroundStyle(.secondary)
}
