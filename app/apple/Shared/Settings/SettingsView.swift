import SwiftUI
import TabToolKit

/// The Settings window: one tab per group of settings.
///
/// Every control writes straight to the stored preference; open documents
/// react to the grid and format bar preferences through their own `@AppStorage`.
/// Each tab form is as tall as its rows, so the window follows the selected tab.
struct SettingsView: View {
    var body: some View {
        TabView {
            GeneralSettings()
                .tabItem { Label("General", systemImage: "gearshape") }
            NewDocumentSettings()
                .tabItem { Label("New Document", systemImage: "doc") }
            GridSettings()
                .tabItem { Label("Grid", systemImage: "tablecells") }
            LargeFileSettings()
                .tabItem { Label("Large Files", systemImage: "externaldrive") }
        }
        .formStyle(.grouped)
        #if os(macOS)
        .frame(width: 460)
        #endif
    }
}

private struct GeneralSettings: View {
    @AppStorage(Preferences.Key.appLanguage) private var language = Preferences.Default.appLanguage
    @AppStorage(Preferences.Key.showFormatBar) private var showFormatBar = Preferences.Default.showFormatBar
    @State private var needsRestart = false

    var body: some View {
        Form {
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
            HStack {
                caption("Takes effect after restart.")
                if needsRestart {
                    Spacer()
                    #if os(macOS)
                    Button { AppRelaunch.request() } label: {
                        Label("Restart Now", systemImage: "arrow.clockwise")
                    }
                    .labelStyle(.iconOnly)
                    .buttonStyle(.borderless)
                    .help("Restart Now")
                    .accessibilityIdentifier("settings-restart")
                    #else
                    caption("Open TabTool again to apply it.")
                    #endif
                }
            }

            Toggle("Show Format Bar", isOn: $showFormatBar)
                .accessibilityIdentifier("settings-show-format-bar")
        }
        .fixedSize(horizontal: false, vertical: true)
    }
}

private struct NewDocumentSettings: View {
    @AppStorage(Preferences.Key.newDocumentEncoding) private var encodingID = Preferences.Default.encodingID
    @AppStorage(Preferences.Key.newDocumentSeparator) private var separator = Preferences.Default.separator
    @AppStorage(Preferences.Key.newDocumentQuoteStyle) private var quoteStyle = Preferences.Default.quoteStyle
    @AppStorage(Preferences.Key.newDocumentDecimalMark) private var decimalMark = Preferences.Default.decimalMark
    @AppStorage(Preferences.Key.newDocumentFirstRowAsHeader)
    private var firstRowAsHeader = Preferences.Default.firstRowAsHeader

    var body: some View {
        Form {
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
        .fixedSize(horizontal: false, vertical: true)
    }
}

private struct GridSettings: View {
    @AppStorage(Preferences.Key.gridAlternatingRows) private var alternatingRows = Preferences.Default.alternatingRows
    @AppStorage(Preferences.Key.gridRowHeight) private var rowHeight = Preferences.Default.rowHeight
    @AppStorage(Preferences.Key.gridShowsRowNumbers) private var showsRowNumbers = Preferences.Default.showsRowNumbers

    var body: some View {
        Form {
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
        .fixedSize(horizontal: false, vertical: true)
    }

    private func rowHeightTitle(_ height: Double) -> LocalizedStringKey {
        switch height {
        case ..<Preferences.Default.rowHeight: "Compact"
        case Preferences.Default.rowHeight: "Default"
        default: "Comfortable"
        }
    }
}

private struct LargeFileSettings: View {
    @AppStorage(Preferences.Key.largeFileThresholdMiB)
    private var largeFileThresholdMiB = Preferences.Default.largeFileThresholdMiB

    var body: some View {
        Form {
            Picker("Large File Threshold", selection: $largeFileThresholdMiB) {
                ForEach(Preferences.largeFileThresholdsMiB, id: \.self) { mebibytes in
                    Text(Int64(mebibytes) * 1_048_576, format: .byteCount(style: .binary)).tag(mebibytes)
                }
            }
            .accessibilityIdentifier("settings-large-threshold")
            caption("Files at or above this size open in large file mode.")
        }
        .fixedSize(horizontal: false, vertical: true)
    }
}

private func caption(_ text: LocalizedStringKey) -> some View {
    Text(text)
        .font(.caption)
        .foregroundStyle(.secondary)
}
