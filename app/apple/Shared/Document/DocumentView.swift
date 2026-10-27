import SwiftUI
import TabToolKit

/// The editor surface: the grid, format controls, status line, and the
/// banners and overlays that explain why the document needs attention.
struct DocumentView: View {
    @ObservedObject var document: CSVDocument
    let fileURL: URL?

    @Environment(\.undoManager) private var undoManager
    @AppStorage(FormatBarVisibility.storageKey) private var showFormatBar = true

    @State private var pendingReparse: CSVConfiguration?
    @State private var isConfirmingReparse = false
    @State private var isReparsing = false
    @State private var formatAlert: FormatAlert?

    var body: some View {
        VStack(spacing: 0) {
            content

            if let issue = document.writeIssue {
                Divider()
                NeedsAttentionBanner(
                    issue: issue,
                    canUndo: undoManager?.canUndo == true,
                    onUndo: { undoManager?.undo() },
                    onShowFormatControls: { showFormatBar = true }
                )
            }

            if showFormatBar {
                Divider()
                FormatBar(
                    document: document,
                    isBusy: isReparsing || document.phase == .loading,
                    isDetectAvailable: document.fileURL != nil,
                    onConfigurationChange: applyConfiguration,
                    onDetectAutomatically: detectAutomatically
                )
            }

            if let table = document.table {
                Divider()
                DocumentStatusBar(table: table, configuration: document.configuration)
            }
        }
        .frame(minWidth: 640, minHeight: 400)
        .focusedSceneObject(document)
        .onAppear {
            document.setUndoManager(undoManager)
        }
        .task {
            document.setFileURL(fileURL)
            await document.load()
        }
        .onDisappear {
            document.cancelLoading()
        }
        .onChange(of: fileURL) { _, newValue in
            // Save As changes the URL without recreating this view.
            document.setFileURL(newValue)
        }
        .onChange(of: isConfirmingReparse) { _, isPresented in
            if !isPresented {
                pendingReparse = nil
            }
        }
        .confirmationDialog(
            "Re-read the file and discard unsaved edits?",
            isPresented: $isConfirmingReparse,
            titleVisibility: .visible,
            presenting: pendingReparse
        ) { configuration in
            Button("Re-read", role: .destructive) {
                startConfirmedReparse(configuration)
            }
            Button("Cancel", role: .cancel) {
                pendingReparse = nil
            }
        } message: { _ in
            Text("The table will be replaced with the last saved version from disk. The file itself is not changed.")
        }
        .alert(
            formatAlert?.title ?? LocalizedStringKey(""),
            isPresented: Binding(
                get: { formatAlert != nil },
                set: { isPresented in
                    if !isPresented {
                        formatAlert = nil
                    }
                }
            ),
            presenting: formatAlert
        ) { _ in
            Button("OK", role: .cancel) {}
        } message: { alert in
            Text(verbatim: alert.message)
        }
    }

    @ViewBuilder
    private var content: some View {
        switch document.phase {
        case .loading:
            ProgressView("Loading…")
                .frame(maxWidth: .infinity, maxHeight: .infinity)
        case .ready, .needsAttention:
            if document.table != nil {
                VStack(spacing: 0) {
                    GridToolbar(document: document)
                    Divider()
                    CSVGrid(document: document, isEditable: true)
                }
            } else {
                ProgressView("Loading…")
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
            }
        case .readFailed, .unreadableFile:
            ErrorOverlay(
                phase: document.phase,
                canDetect: document.fileURL != nil && !isReparsing,
                onDetectAutomatically: detectAutomatically
            )
        }
    }

    /// Applies one format-bar edit through the planner.
    private func applyConfiguration(_ configuration: CSVConfiguration) {
        guard !isReparsing else { return }
        switch FormatChangePlanner.plan(from: document.configuration,
                                       to: configuration) {
        case .none:
            break
        case .decimalMark(let mark):
            if document.table == nil {
                startReparse(configuration: configuration)
            } else {
                document.setDecimalMark(mark)
            }
        case .header(let enabled):
            if document.table == nil {
                startReparse(configuration: configuration)
            } else {
                document.setHeaderEnabled(enabled)
            }
        case .reparse(let configuration):
            startReparse(configuration: configuration)
        }
    }

    /// Re-reads the file after an explicit format change.
    ///
    /// The dirty check runs off the main actor. Only an actually dirty
    /// document asks to discard edits; a re-read rejected because it raced
    /// with another change is reported as a retry instead.
    private func startReparse(configuration: CSVConfiguration) {
        guard !isReparsing else { return }
        isReparsing = true
        Task {
            await continueReparse(to: configuration)
            isReparsing = false
        }
    }

    private func continueReparse(to configuration: CSVConfiguration) async {
        guard document.fileURL != nil, document.table != nil else {
            await applyReparse(to: configuration)
            return
        }
        if await document.hasUnsavedChangesAsync() {
            pendingReparse = configuration
            isConfirmingReparse = true
            return
        }
        await applyReparse(to: configuration)
    }

    private func startConfirmedReparse(_ configuration: CSVConfiguration) {
        pendingReparse = nil
        isReparsing = true
        Task {
            await applyReparse(to: configuration)
            isReparsing = false
        }
    }

    private func applyReparse(to configuration: CSVConfiguration) async {
        do {
            try await document.applyParsingConfiguration(configuration, force: true)
        } catch DocumentError.requiresConfirmation {
            formatAlert = FormatAlert(
                title: "Re-read Interrupted",
                message: String(localized: "The document changed while it was being re-read, so the new format was not applied. Try again.")
            )
        } catch {
            formatAlert = FormatAlert(
                title: "Format Not Applied",
                message: FormatErrorText.alertMessage(for: error)
            )
        }
    }

    private func detectAutomatically() {
        guard !isReparsing, document.fileURL != nil else { return }
        isReparsing = true
        Task {
            do {
                let detected = try await document.detectFormatFromDisk()
                if document.table == nil || detected != document.configuration {
                    await continueReparse(to: detected)
                }
            } catch {
                formatAlert = FormatAlert(
                    title: "Format Not Detected",
                    message: FormatErrorText.alertMessage(for: error)
                )
            }
            isReparsing = false
        }
    }
}

private struct FormatAlert: Identifiable {
    let id = UUID()
    let title: LocalizedStringKey
    let message: String
}

/// Row and column actions above the grid.
///
/// The primary action inserts below or to the right of the selection; the
/// context menu offers the opposite side. A new row or column becomes the
/// selection.
private struct GridToolbar: View {
    @ObservedObject var document: CSVDocument

    var body: some View {
        HStack(spacing: 2) {
            Button {
                document.addRowBelowSelection()
            } label: {
                Label("Add Row Below", systemImage: "plus.rectangle")
                    .labelStyle(.iconOnly)
            }
            .help("Add Row Below")
            .accessibilityLabel("Add Row Below")
            .accessibilityIdentifier("add-row")
            .contextMenu {
                Button("Add Row Above") {
                    document.addRowAboveSelection()
                }
            }

            Button {
                document.addColumnRightOfSelection()
            } label: {
                Label("Add Column Right", systemImage: "plus.rectangle.portrait")
                    .labelStyle(.iconOnly)
            }
            .help("Add Column Right")
            .accessibilityLabel("Add Column Right")
            .accessibilityIdentifier("add-column")
            .contextMenu {
                Button("Add Column Left") {
                    document.addColumnLeftOfSelection()
                }
            }

            Divider()
                .frame(height: 16)
                .padding(.horizontal, 4)

            Button(role: .destructive) {
                guard document.canDeleteSelection else { return }
                document.deleteSelection()
            } label: {
                Label("Delete Selection", systemImage: "trash")
                    .labelStyle(.iconOnly)
            }
            .help("Delete Selected Rows or Columns")
            .accessibilityLabel("Delete Selection")
            .accessibilityIdentifier("delete-selection")
            .disabled(!document.canDeleteSelection)

            Spacer()
        }
        .padding(.horizontal, 8)
        .padding(.vertical, 4)
    }
}
