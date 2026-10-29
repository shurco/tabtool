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
        #expect(url.pathExtension == "csv")
        #expect(text.contains("apple"))
        try? FileManager.default.removeItem(at: url)
    }
}
