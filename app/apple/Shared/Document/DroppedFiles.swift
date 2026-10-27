import SwiftUI
import UniformTypeIdentifiers

/// Table files the app accepts when they are dropped on a document window.
enum DroppedFiles {
    /// Extensions declared in `Info.plist`; keep both in sync.
    static let tableExtensions: Set<String> = ["csv", "tsv", "txt", "tab", "dat"]

    /// Content types the document can read: the known table types plus the
    /// extra extensions, which have no system type of their own.
    static var readableContentTypes: [UTType] {
        [.commaSeparatedText, .tabSeparatedText, .plainText]
            + ["tab", "dat"].compactMap { UTType(filenameExtension: $0) }
    }

    /// Keeps only local file URLs with a table extension.
    static func tableFiles(in urls: [URL]) -> [URL] {
        urls.filter { $0.isFileURL && tableExtensions.contains($0.pathExtension.lowercased()) }
    }
}

private struct OpenDroppedFiles: ViewModifier {
    @Environment(\.openDocument) private var openDocument
    @State private var isTargeted = false

    func body(content: Content) -> some View {
        content
            .dropDestination(for: URL.self) { urls, _ in
                let files = DroppedFiles.tableFiles(in: urls)
                guard !files.isEmpty else { return false }
                Task {
                    for file in files {
                        try? await openDocument(at: file)
                    }
                }
                return true
            } isTargeted: { isTargeted = $0 }
            .overlay {
                if isTargeted {
                    Rectangle()
                        .strokeBorder(Color.accentColor, lineWidth: 3)
                        .allowsHitTesting(false)
                }
            }
    }
}

extension View {
    /// Opens dropped table files as documents.
    func opensDroppedTableFiles() -> some View {
        modifier(OpenDroppedFiles())
    }
}
