import SwiftUI
import TabToolKit

/// The editor surface: the grid, format controls, status line, and the
/// banners and overlays that explain why the document needs attention.
struct DocumentView: View {
    @ObservedObject var document: CSVDocument
    let fileURL: URL?

    @Environment(\.undoManager) private var undoManager
    #if os(iOS)
    @Environment(\.scenePhase) private var scenePhase
    @Environment(\.horizontalSizeClass) private var horizontalSizeClass
    #endif
    @AppStorage(Preferences.Key.showFormatBar) private var showFormatBar = Preferences.Default.showFormatBar

    @State private var pendingReparse: CSVConfiguration?
    @State private var isConfirmingReparse = false
    @State private var isReparsing = false
    @State private var formatAlert: FormatAlert?
    #if os(iOS)
    @State private var showsSettings = false
    @State private var showsAbout = false
    #endif

    var body: some View {
        VStack(spacing: 0) {
            content

            if document.isLargeMode, document.hasContent {
                Divider()
                LargeFileBanner()
                if case .stale(let appended) = document.largeStatus {
                    Divider()
                    LargeFileStaleBanner(appended: appended) {
                        Task { await document.reloadLargeFile() }
                    }
                }
            }

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
                scrollingOnCompact {
                    FormatBar(
                        document: document,
                        isBusy: isReparsing || document.phase == .loading || document.isIndexing
                            || document.isSaving,
                        isDetectAvailable: document.fileURL != nil,
                        onConfigurationChange: applyConfiguration,
                        onDetectAutomatically: detectAutomatically
                    )
                }
            }

            if let source = document.rowSource {
                Divider()
                DocumentStatusBar(table: source, configuration: document.configuration,
                                  visibleRowCount: document.rowFilter.visibleRows?.count,
                                  status: document.largeStatus, onCancelSave: document.cancelLargeSave)
            }
        }
        #if os(macOS)
        .frame(minWidth: 640, minHeight: 400)
        .opensDroppedTableFiles()
        #endif
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
        #if os(macOS)
        .onReceive(NotificationCenter.default.publisher(for: NSWindow.didBecomeKeyNotification)) { _ in
            document.refreshLargeStaleness()
        }
        #else
        .onChange(of: scenePhase) { _, phase in
            if phase == .active {
                document.refreshLargeStaleness()
            }
        }
        #endif
        .onChange(of: fileURL) { _, newValue in
            // Save As changes the URL without recreating this view.
            document.setFileURL(newValue)
        }
        .onChange(of: document.notice) { _, notice in
            guard let notice else { return }
            formatAlert = FormatAlert(title: "Edit Not Applied", message: Self.message(for: notice))
            document.notice = nil
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
        #if os(iOS)
        .toolbar {
            ToolbarItemGroup(placement: .topBarLeading) {
                Button("Undo", systemImage: "arrow.uturn.backward") {
                    undoManager?.undo()
                }
                .disabled(!(undoManager?.canUndo ?? false))
                .accessibilityIdentifier("undo")
                Button("Redo", systemImage: "arrow.uturn.forward") {
                    undoManager?.redo()
                }
                .disabled(!(undoManager?.canRedo ?? false))
                .accessibilityIdentifier("redo")
            }
            ToolbarItem(placement: .primaryAction) {
                Button("Settings", systemImage: "gearshape") {
                    showsSettings = true
                }
                .accessibilityIdentifier("settings")
            }
        }
        .sheet(isPresented: $showsSettings) {
            NavigationStack {
                SettingsView()
                    .navigationTitle("Settings")
                    .toolbar {
                        ToolbarItem(placement: .cancellationAction) {
                            Button("About") { showsAbout = true }
                        }
                        ToolbarItem(placement: .confirmationAction) {
                            Button("Done") { showsSettings = false }
                        }
                    }
                    .navigationDestination(isPresented: $showsAbout) {
                        AboutView()
                    }
            }
        }
        #endif
    }

    /// Large files stay read-only on iOS until they can be saved.
    private var cellsAreEditable: Bool {
        #if os(iOS)
        document.canEditCells && !document.isLargeMode
        #else
        document.canEditCells
        #endif
    }

    /// Lets a wide format or filter bar slide sideways on a narrow phone.
    @ViewBuilder
    private func scrollingOnCompact<Content: View>(@ViewBuilder content: () -> Content) -> some View {
        #if os(iOS)
        if horizontalSizeClass == .compact {
            ScrollView(.horizontal, showsIndicators: false) { content() }
        } else {
            content()
        }
        #else
        content()
        #endif
    }

    private static func message(for notice: DocumentNotice) -> String {
        switch notice {
        case .editRejected(let error):
            return FormatErrorText.alertMessage(for: error)
        case .copyLimitExceeded:
            return String(localized: "Too many cells selected to copy (limit 1,000,000).")
        }
    }

    @ViewBuilder
    private var content: some View {
        switch document.phase {
        case .loading:
            ProgressView("Loading…")
                .frame(maxWidth: .infinity, maxHeight: .infinity)
        case .ready, .needsAttention:
            if document.hasContent {
                VStack(spacing: 0) {
                    GridToolbar(document: document)
                    Divider()
                    if document.canFilter, document.rowFilter.isBarVisible {
                        scrollingOnCompact {
                            RowFilterBar(document: document)
                        }
                        Divider()
                    }
                    CSVGrid(document: document, isEditable: cellsAreEditable,
                            allowsStructuralEdits: !document.isLargeMode)
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
            if !document.hasContent {
                startReparse(configuration: configuration)
            } else {
                document.setDecimalMark(mark)
            }
        case .header(let enabled):
            if !document.hasContent {
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
        guard document.fileURL != nil, document.hasContent else {
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
                if !document.hasContent || detected != document.configuration {
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
