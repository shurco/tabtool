import Foundation
import Testing
@testable import TabTool

@MainActor
@Suite struct DocumentLibraryTests {
    private func makeDirectory() throws -> URL {
        let url = FileManager.default.temporaryDirectory
            .appendingPathComponent("library-\(UUID().uuidString)", isDirectory: true)
        try FileManager.default.createDirectory(at: url, withIntermediateDirectories: true)
        return url
    }

    private func write(_ name: String, in directory: URL) throws {
        try Data("a,b\n".utf8).write(to: directory.appendingPathComponent(name))
    }

    @Test func listsOnlySupportedFilesNewestFirst() throws {
        let directory = try makeDirectory()
        try write("old.csv", in: directory)
        try write("new.tsv", in: directory)
        try write("notes.md", in: directory)
        let old = directory.appendingPathComponent("old.csv")
        try FileManager.default.setAttributes([.modificationDate: Date(timeIntervalSince1970: 100)],
                                              ofItemAtPath: old.path)

        let library = DocumentLibrary(directory: directory)

        #expect(library.files.map(\.name) == ["new.tsv", "old.csv"])
    }

    @Test func createsAnEmptyDocumentAndListsIt() async throws {
        let directory = try makeDirectory()
        let library = DocumentLibrary(directory: directory)

        let file = try await library.createDocument()

        #expect(file.name == "Untitled.csv")
        #expect(FileManager.default.fileExists(atPath: file.url.path))
        #expect(library.files.map(\.name) == ["Untitled.csv"])

        let reopened = CSVDocument(data: try Data(contentsOf: file.url))
        await reopened.load()
        #expect(reopened.hasContent)
    }

    @Test func aSecondNewDocumentCountsUp() async throws {
        let directory = try makeDirectory()
        let library = DocumentLibrary(directory: directory)

        _ = try await library.createDocument()
        let second = try await library.createDocument()

        #expect(second.name == "Untitled 2.csv")
    }

    @Test func importsUnderAFreeName() throws {
        let directory = try makeDirectory()
        let source = FileManager.default.temporaryDirectory
            .appendingPathComponent("import-\(UUID().uuidString).csv")
        try Data("x,y\n".utf8).write(to: source)
        defer { try? FileManager.default.removeItem(at: source) }
        try write("import.csv", in: directory)
        let library = DocumentLibrary(directory: directory)

        let imported = try library.importDocument(from: source)

        #expect(imported.name.hasPrefix("import-"))
        #expect(FileManager.default.fileExists(atPath: imported.url.path))
    }

    @Test func renamesAndDeletes() async throws {
        let directory = try makeDirectory()
        let library = DocumentLibrary(directory: directory)
        let file = try await library.createDocument()

        let renamed = try library.rename(file, to: "Budget")
        #expect(renamed.name == "Budget.csv")
        #expect(library.files.map(\.name) == ["Budget.csv"])

        try library.delete(renamed)
        #expect(library.files.isEmpty)
    }
}
