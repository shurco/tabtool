import Foundation
import Testing
import TabToolKit
@testable import TabTool

@MainActor
@Suite struct IOSExportTests {
    @Test func prepareWritesATemporaryCSV() throws {
        let table = CSVTable(rows: [["apple"]], headers: ["name"])
        let configuration = CSVConfiguration(firstRowAsHeader: true)
        let url = try ExportCoordinator.prepare(
            table: table, documentConfiguration: configuration,
            configuration: configuration, baseName: "Sales-export")
        let text = try String(contentsOf: url, encoding: .utf8)
        let directory = url.deletingLastPathComponent()
        #expect(url.pathExtension == "csv")
        #expect(text.contains("apple"))
        ExportCoordinator.removeTemporaryExport(url)
        #expect(!FileManager.default.fileExists(atPath: directory.path))
    }

    @Test func removeTemporaryExportLeavesAFileInTheSystemTemporaryDirectory() throws {
        let url = FileManager.default.temporaryDirectory
            .appendingPathComponent("tabtool-export-guard-\(UUID().uuidString).csv")
        try Data("a".utf8).write(to: url)
        ExportCoordinator.removeTemporaryExport(url)
        #expect(FileManager.default.fileExists(atPath: url.path))
        try? FileManager.default.removeItem(at: url)
    }
}
