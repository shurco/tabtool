import SwiftUI
import UniformTypeIdentifiers

/// The app's home: the documents in its folder, with creation, import,
/// rename and delete.
struct DocumentLibraryView: View {
    @StateObject private var library = DocumentLibrary()
    @State private var showsSettings = false
    @State private var isImporting = false
    @State private var errorMessage: String?
    @State private var pendingDelete: DocumentFile?
    @State private var renaming: DocumentFile?
    @State private var renameText = ""
    @State private var path = NavigationPath()

    var body: some View {
        NavigationStack(path: $path) {
            content
                .navigationTitle("Documents")
                .toolbar {
                    ToolbarItem(placement: .topBarLeading) {
                        Button("Settings", systemImage: "gearshape") { showsSettings = true }
                            .accessibilityIdentifier("settings")
                    }
                    ToolbarItem(placement: .topBarTrailing) {
                        Menu {
                            Button("New Document", systemImage: "doc.badge.plus") {
                                createDocument()
                            }
                            .accessibilityIdentifier("new-document")
                            Button("Import…", systemImage: "square.and.arrow.down") {
                                isImporting = true
                            }
                            .accessibilityIdentifier("import-document")
                        } label: {
                            Label("Add Document", systemImage: "plus")
                        }
                        .accessibilityIdentifier("add-document")
                    }
                }
                .navigationDestination(for: DocumentFile.self) { file in
                    DocumentEditorScreen(file: file)
                }
        }
        .onAppear { ReviewPrompt.recordLaunch() }
        .sheet(isPresented: $showsSettings) { IOSSettingsView() }
        .fileImporter(isPresented: $isImporting,
                      allowedContentTypes: [.commaSeparatedText, .tabSeparatedText, .plainText],
                      allowsMultipleSelection: false) { result in
            switch result {
            case .success(let urls):
                guard let url = urls.first else { return }
                do {
                    let imported = try library.importDocument(from: url)
                    open(imported)
                } catch {
                    errorMessage = error.localizedDescription
                }
            case .failure(let error):
                errorMessage = error.localizedDescription
            }
        }
        .confirmationDialog(
            "Delete Document?",
            isPresented: Binding(get: { pendingDelete != nil },
                                 set: { if !$0 { pendingDelete = nil } }),
            titleVisibility: .visible,
            presenting: pendingDelete
        ) { file in
            Button("Delete", role: .destructive) {
                do {
                    try library.delete(file)
                } catch {
                    errorMessage = error.localizedDescription
                }
                pendingDelete = nil
            }
            Button("Cancel", role: .cancel) { pendingDelete = nil }
        } message: { file in
            Text(verbatim: file.name)
        }
        .alert("Rename", isPresented: Binding(get: { renaming != nil },
                                              set: { if !$0 { renaming = nil } })) {
            TextField("Name", text: $renameText)
            Button("Cancel", role: .cancel) { renaming = nil }
            Button("Rename") {
                guard let file = renaming else { return }
                do {
                    try library.rename(file, to: renameText)
                } catch {
                    errorMessage = error.localizedDescription
                }
                renaming = nil
            }
        }
        .alert("Something Went Wrong", isPresented: Binding(
            get: { errorMessage != nil },
            set: { if !$0 { errorMessage = nil } }
        )) {
            Button("OK", role: .cancel) { errorMessage = nil }
        } message: {
            Text(verbatim: errorMessage ?? "")
        }
    }

    @ViewBuilder
    private var content: some View {
        if library.files.isEmpty {
            ContentUnavailableView(
                "No Documents",
                systemImage: "tablecells",
                description: Text("Create a document or import a CSV file.")
            )
        } else {
            List {
                ForEach(library.files) { file in
                    NavigationLink(value: file) {
                        row(for: file)
                    }
                    .swipeActions {
                        Button("Delete", systemImage: "trash", role: .destructive) {
                            pendingDelete = file
                        }
                    }
                    .contextMenu {
                        Button("Rename", systemImage: "pencil") {
                            renameText = file.name
                            renaming = file
                        }
                        Button("Delete", systemImage: "trash", role: .destructive) {
                            pendingDelete = file
                        }
                    }
                }
            }
            .listStyle(.plain)
            .accessibilityIdentifier("document-list")
        }
    }

    private func row(for file: DocumentFile) -> some View {
        VStack(alignment: .leading, spacing: 2) {
            Text(verbatim: file.url.deletingPathExtension().lastPathComponent)
                .font(.body)
            Text(verbatim: subtitle(for: file))
                .font(.caption)
                .foregroundStyle(.secondary)
        }
        .padding(.vertical, 2)
    }

    private func subtitle(for file: DocumentFile) -> String {
        let size = file.size.formatted(.byteCount(style: .file))
        return "\(size) · \(file.modified.formatted(date: .abbreviated, time: .shortened))"
    }

    private func createDocument() {
        Task {
            do {
                open(try await library.createDocument())
            } catch {
                errorMessage = error.localizedDescription
            }
        }
    }

    /// Opening is a push, so the editor path is the library's navigation path.
    private func open(_ file: DocumentFile) {
        path.append(file)
    }
}
