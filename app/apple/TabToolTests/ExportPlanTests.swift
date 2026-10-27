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

    /// The panel owns the final URL and its replace confirmation.
    ///
    /// Writing to a rewritten extension could overwrite an existing file
    /// without asking.
    @Test func exportWritesThePanelURLWithoutRewritingTheExtension() throws {
        let directory = FileManager.default.temporaryDirectory
            .appendingPathComponent("ExportPlanTests-\(UUID().uuidString)",
                                    isDirectory: true)
        try FileManager.default.createDirectory(at: directory,
                                                withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }

        let panelURL = directory.appendingPathComponent("notes.txt")
        let sibling = directory.appendingPathComponent("notes.csv")
        try "keep".write(to: sibling, atomically: true, encoding: .utf8)

        let table = CSVTable(rows: [["a", "b"]], headers: nil)
        try ExportPlan.write(table: table, documentConfiguration: .init(),
                             configuration: .init(columnSeparator: ";"),
                             to: panelURL)

        #expect(FileManager.default.fileExists(atPath: panelURL.path))
        #expect(try String(contentsOf: sibling, encoding: .utf8) == "keep")
        #expect(try String(contentsOf: panelURL, encoding: .utf8) == "\"a\";\"b\"")
    }

    @Test func convertedDecimalAmbiguityAsksForConfirmation() {
        let table = CSVTable(rows: [["1.5"]], headers: nil)
        let document = CSVConfiguration(columnSeparator: ",", decimalMark: ".")
        let export = CSVConfiguration(columnSeparator: ",", quoteCharacter: "",
                                      escapeCharacter: "", decimalMark: ",")
        #expect(ExportPlan.decision(table: table,
                                    documentConfiguration: document,
                                    exportConfiguration: export)
            == .confirmAmbiguous)
    }

    @Test func convertedDecimalOutputIsNotBlockedByOriginalAmbiguity() {
        let table = CSVTable(rows: [["1,5"]], headers: nil)
        let document = CSVConfiguration(columnSeparator: ",", quoteCharacter: "",
                                        escapeCharacter: "", decimalMark: ",")
        let export = CSVConfiguration(columnSeparator: ",", quoteCharacter: "",
                                      escapeCharacter: "", decimalMark: ".")
        #expect(ExportPlan.decision(table: table,
                                    documentConfiguration: document,
                                    exportConfiguration: export) == .proceed)
    }

    @Test func convertedAmbiguityRequiresConfirmationBeforeWriting() throws {
        let table = CSVTable(rows: [["1.5"]], headers: nil)
        let document = CSVConfiguration(columnSeparator: ",", decimalMark: ".")
        let export = CSVConfiguration(columnSeparator: ",", quoteCharacter: "",
                                      escapeCharacter: "", decimalMark: ",")
        let url = temporaryURL()
        defer { try? FileManager.default.removeItem(at: url) }

        #expect(ExportPlan.decision(table: table,
                                    documentConfiguration: document,
                                    exportConfiguration: export)
            == .confirmAmbiguous)
        #expect(throws: CSVError.ambiguousOutput) {
            try ExportPlan.write(table: table, documentConfiguration: document,
                                 configuration: export, to: url)
        }
        #expect(!FileManager.default.fileExists(atPath: url.path))

        try ExportPlan.write(table: table, documentConfiguration: document,
                             configuration: export, to: url,
                             allowAmbiguousOutput: true)
        #expect(String(data: try Data(contentsOf: url), encoding: .utf8) == "1,5")
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
