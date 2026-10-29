import Foundation
import TabToolKit

/// The documents in the app's folder: listing, creation, import, rename and
/// delete. The library owns the file side only; document content is the
/// editor's business.
@MainActor
final class DocumentLibrary: ObservableObject {
    @Published private(set) var files: [DocumentFile] = []

    let directory: URL
    private let fileManager: FileManager

    /// The extensions the library lists. Files are created as CSV; TSV and
    /// plain text are listed so an imported file stays reachable.
    static let listedExtensions: Set<String> = ["csv", "tsv", "txt"]

    init(directory: URL? = nil, fileManager: FileManager = .default) {
        self.fileManager = fileManager
        self.directory = directory ?? fileManager.urls(for: .documentDirectory, in: .userDomainMask)[0]
        refresh()
    }

    func refresh() {
        let keys: [URLResourceKey] = [.contentModificationDateKey, .fileSizeKey, .isRegularFileKey]
        let contents = (try? fileManager.contentsOfDirectory(
            at: directory, includingPropertiesForKeys: keys, options: [.skipsHiddenFiles])) ?? []
        files = contents.compactMap { url in
            guard Self.listedExtensions.contains(url.pathExtension.lowercased()),
                  let values = try? url.resourceValues(forKeys: Set(keys)),
                  values.isRegularFile == true else { return nil }
            return DocumentFile(url: url,
                                modified: values.contentModificationDate ?? .distantPast,
                                size: Int64(values.fileSize ?? 0))
        }
        .sorted { lhs, rhs in
            lhs.modified == rhs.modified ? lhs.name < rhs.name : lhs.modified > rhs.modified
        }
    }

    /// Creates an empty document in the current new-document format and
    /// returns it, ready to open.
    func createDocument() async throws -> DocumentFile {
        let name = DocumentNaming.newDocumentName(existing: existingNames())
        let url = directory.appendingPathComponent(name)
        let document = CSVDocument(newDocumentConfiguration: { Preferences.newDocumentConfiguration() })
        await document.load()
        document.setFileURL(url)
        try document.encodedData().write(to: url, options: .atomic)
        refresh()
        return documentFile(at: url)
    }

    /// Copies a file the user picked into the library under a free name.
    func importDocument(from url: URL) throws -> DocumentFile {
        let scoped = url.startAccessingSecurityScopedResource()
        defer { if scoped { url.stopAccessingSecurityScopedResource() } }
        let name = DocumentNaming.availableName(for: url.lastPathComponent, existing: existingNames())
        let destination = directory.appendingPathComponent(name)
        try fileManager.copyItem(at: url, to: destination)
        refresh()
        return documentFile(at: destination)
    }

    func delete(_ file: DocumentFile) throws {
        try fileManager.removeItem(at: file.url)
        refresh()
    }

    @discardableResult
    func rename(_ file: DocumentFile, to proposed: String) throws -> DocumentFile {
        let trimmed = proposed.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { return file }
        let pathExtension = file.url.pathExtension
        let name = trimmed.contains(".") ? trimmed : "\(trimmed).\(pathExtension)"
        guard name != file.name else { return file }
        let available = DocumentNaming.availableName(for: name, existing: existingNames())
        let destination = directory.appendingPathComponent(available)
        try fileManager.moveItem(at: file.url, to: destination)
        refresh()
        return documentFile(at: destination)
    }

    func existingNames() -> Set<String> {
        let names = (try? fileManager.contentsOfDirectory(atPath: directory.path)) ?? []
        return Set(names.map { $0.lowercased() })
    }

    private func documentFile(at url: URL) -> DocumentFile {
        files.first { $0.url == url }
            ?? DocumentFile(url: url, modified: Date(), size: 0)
    }
}
