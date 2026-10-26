import Foundation
import Testing
import TabToolKit
import UniformTypeIdentifiers
@testable import TabTool

@Suite struct ExportPlanTests {
    @Test func baseNameAppendsExportSuffix() {
        let url = URL(fileURLWithPath: "/tmp/Sales.csv")
        #expect(ExportPlan.baseName(forDocumentAt: url) == "Sales-export")
    }

    @Test func baseNameFallsBackToUntitled() {
        #expect(ExportPlan.baseName(forDocumentAt: nil) == "Untitled-export")
    }

    @Test func filenameUsesCSVForNonTabSeparators() {
        for separator in [",", ";", "|"] {
            let configuration = CSVConfiguration(columnSeparator: separator)
            #expect(ExportPlan.filename(baseName: "Sales-export",
                                        configuration: configuration)
                == "Sales-export.csv")
        }
    }

    @Test func filenameUsesTSVForTabSeparator() {
        let configuration = CSVConfiguration(columnSeparator: "\t")
        #expect(ExportPlan.filename(baseName: "Sales-export",
                                    configuration: configuration)
            == "Sales-export.tsv")
    }

    @Test func contentTypeFollowsSeparator() {
        #expect(ExportPlan.contentType(for: .init(columnSeparator: ","))
            == UTType.commaSeparatedText)
        #expect(ExportPlan.contentType(for: .init(columnSeparator: ";"))
            == UTType.commaSeparatedText)
        #expect(ExportPlan.contentType(for: .init(columnSeparator: "\t"))
            == UTType.tabSeparatedText)
    }

    @Test func renamingSwapsOnlyTheExtension() {
        let tab = CSVConfiguration(columnSeparator: "\t")
        let comma = CSVConfiguration(columnSeparator: ",")
        #expect(ExportPlan.renaming("Sales-export.csv", configuration: tab)
            == "Sales-export.tsv")
        #expect(ExportPlan.renaming("My Report", configuration: tab)
            == "My Report.tsv")
        #expect(ExportPlan.renaming("Sales-export.tsv", configuration: comma)
            == "Sales-export.csv")
    }

    @Test func outputURLUsesTheConfigurationExtension() {
        let tab = CSVConfiguration(columnSeparator: "\t")
        let comma = CSVConfiguration(columnSeparator: ",")
        #expect(ExportPlan.outputURL(URL(fileURLWithPath: "/tmp/out.csv"),
                                     configuration: tab).path == "/tmp/out.tsv")
        #expect(ExportPlan.outputURL(URL(fileURLWithPath: "/tmp/out"),
                                     configuration: tab).path == "/tmp/out.tsv")
        #expect(ExportPlan.outputURL(URL(fileURLWithPath: "/tmp/out.tsv"),
                                     configuration: comma).path == "/tmp/out.csv")
    }

    @Test func headerFollowsDocumentHeaders() {
        let table = CSVTable(rows: [["x"]], headers: ["Name"])
        var configuration = CSVConfiguration()
        configuration.firstRowAsHeader = true
        #expect(ExportPlan.header(table: table, configuration: configuration)
            == ["Name"])
    }

    @Test func headerFallsBackToGeneratedNames() {
        let table = CSVTable(rows: [["x", "y"]], headers: nil)
        var configuration = CSVConfiguration()
        configuration.firstRowAsHeader = true
        #expect(ExportPlan.header(table: table, configuration: configuration)
            == ["A", "B"])
    }

    @Test func headerIsOmittedWhenDisabled() {
        let table = CSVTable(rows: [["x"]], headers: ["Name"])
        #expect(ExportPlan.header(table: table, configuration: .init()) == nil)
    }

    @Test func generatedHeaderMatchesExportOutput() throws {
        let table = CSVTable(rows: [["1,5", "text"]], headers: nil)
        var configuration = CSVConfiguration(decimalMark: ".")
        configuration.firstRowAsHeader = true
        let data = try CSVExport.data(
            table: table,
            documentConfiguration: CSVConfiguration(decimalMark: ","),
            exportConfiguration: configuration
        )
        let text = try #require(String(data: data, encoding: .utf8))
        #expect(ExportPlan.header(table: table, configuration: configuration)
            == ["A", "B"])
        #expect(text.hasPrefix("\"A\",\"B\"\n"))
    }

    @Test func cleanOutputExportsWithoutConfirmation() {
        #expect(ExportPlan.decision(for: nil) == .proceed)
    }

    @Test func ambiguousOutputRequiresConfirmation() {
        #expect(ExportPlan.decision(for: .ambiguousOutput) == .confirmAmbiguous)
    }

    @Test func unencodableOutputIsBlocked() {
        #expect(ExportPlan.decision(for: .unencodableText) == .blockedUnencodable)
    }

    @Test func writingAndReimportingPreservesValuesAndHeader() throws {
        let table = CSVTable(
            rows: [["Alice", "1.50", "hello"], ["Bob", "2.00", "world"]],
            headers: ["Name", "Amount", "Note"]
        )
        let document = CSVConfiguration(columnSeparator: ",", decimalMark: ".",
                                        firstRowAsHeader: true)
        let export = CSVConfiguration(columnSeparator: ";", decimalMark: ",",
                                      firstRowAsHeader: true)
        let url = temporaryURL()
        defer { try? FileManager.default.removeItem(at: url) }

        try ExportPlan.write(table: table, documentConfiguration: document,
                             configuration: export, to: url)

        let data = try Data(contentsOf: url)
        let text = try #require(String(data: data, encoding: .utf8))
        #expect(text == "\"Name\";\"Amount\";\"Note\"\n"
            + "\"Alice\";\"1,50\";\"hello\"\n"
            + "\"Bob\";\"2,00\";\"world\"")

        let parsed = try CSVDocumentIO.parse(data: data, configuration: export)
        #expect(parsed.table.headers == ["Name", "Amount", "Note"])
        #expect(parsed.table.rowCount == 2)
        #expect(parsed.table.cell(row: 0, column: 1) == "1,50")
        #expect(parsed.table.cell(row: 1, column: 1) == "2,00")
        // Export works on a copy.
        #expect(table.cell(row: 0, column: 1) == "1.50")
    }

    @Test func ambiguousExportDoesNotWriteAFile() throws {
        let table = CSVTable(rows: [["a;b"]], headers: nil)
        let export = CSVConfiguration(columnSeparator: ";", quoteCharacter: "")
        let url = temporaryURL()
        defer { try? FileManager.default.removeItem(at: url) }

        #expect(throws: CSVError.ambiguousOutput) {
            try ExportPlan.write(table: table, documentConfiguration: .init(),
                                 configuration: export, to: url)
        }
        #expect(!FileManager.default.fileExists(atPath: url.path))
    }

    @Test func confirmedAmbiguousExportWritesAFile() throws {
        let table = CSVTable(rows: [["a;b"]], headers: nil)
        let export = CSVConfiguration(columnSeparator: ";", quoteCharacter: "")
        let url = temporaryURL()
        defer { try? FileManager.default.removeItem(at: url) }

        try ExportPlan.write(table: table, documentConfiguration: .init(),
                             configuration: export, to: url,
                             allowAmbiguousOutput: true)
        #expect(String(data: try Data(contentsOf: url), encoding: .utf8) == "a;b")
    }

    @Test func unencodableExportDoesNotWriteAFile() throws {
        let table = CSVTable(rows: [["ж"]], headers: nil)
        let export = CSVConfiguration(encoding: .ascii)
        let url = temporaryURL()
        defer { try? FileManager.default.removeItem(at: url) }

        #expect(throws: CSVError.unencodableText) {
            try ExportPlan.write(table: table, documentConfiguration: .init(),
                                 configuration: export, to: url)
        }
        #expect(!FileManager.default.fileExists(atPath: url.path))
    }

    private func temporaryURL() -> URL {
        FileManager.default.temporaryDirectory
            .appendingPathComponent(UUID().uuidString)
            .appendingPathExtension("csv")
    }
}
