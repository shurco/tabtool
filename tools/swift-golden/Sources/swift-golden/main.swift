import Foundation
import TabToolKit

// Generates the cross-language golden files from the Apple TabToolKit
// implementation. Arguments: <fixtures-root> <output-file>. Foundation reads
// the locale from the automatic `-AppleLocale` / `-AppleLanguages` arguments.

enum GoldenToolError: Error, CustomStringConvertible {
    case usage
    case unknownEncoding

    var description: String {
        switch self {
        case .usage: "usage: swift-golden <fixtures-root> <output-file> [-AppleLocale <id>]"
        case .unknownEncoding: "encoding is not in SupportedEncodings.all"
        }
    }
}

func encodingID(_ encoding: String.Encoding) throws -> Int {
    guard let index = SupportedEncodings.all.firstIndex(where: { $0.encoding == encoding }) else {
        throw GoldenToolError.unknownEncoding
    }
    return index
}

func configurationObject(_ configuration: CSVConfiguration) throws -> [String: Any] {
    [
        "encoding": try encodingID(configuration.encoding),
        "separator": configuration.columnSeparator,
        "quote": configuration.quoteCharacter,
        "escape": configuration.escapeCharacter,
        "decimalMark": configuration.decimalMark,
        "header": configuration.firstRowAsHeader,
    ]
}

func skipName(for error: Error) -> String {
    guard let csvError = error as? CSVError else {
        return "\(error)"
    }
    switch csvError {
    case .cannotDecodeData: return "cannotDecodeData"
    case .unencodableText: return "unencodableText"
    case .ambiguousOutput: return "ambiguousOutput"
    default: return "\(csvError)"
    }
}

func fixtureNames(in root: URL) throws -> [String] {
    var names: [String] = []
    for folder in ["Reading", "Heuristic"] {
        let directory = root.appendingPathComponent(folder, isDirectory: true)
        let files = try FileManager.default
            .contentsOfDirectory(at: directory, includingPropertiesForKeys: nil)
            .filter { $0.pathExtension == "csv" }
            .map(\.lastPathComponent)
            .sorted()
        names.append(contentsOf: files.map { "\(folder)/\($0)" })
    }
    return names
}

func fixtureObject(name: String, fixturesRoot: URL) throws -> [String: Any] {
    let data = try Data(contentsOf: fixturesRoot.appendingPathComponent(name))
    let detected = CSVFormatDetector(data: data).detect()
    var parser = try CSVParser(data: data, configuration: detected)
    let hadUTF8BOM = parser.hadUTF8BOM
    let rows = try parser.readAll()
    let table = CSVTable(rows: rows, headers: nil)

    // The detected configuration first, then explicit Windows and Unicode
    // writes. A write is skipped when the fixture bytes are not valid in that
    // encoding, which is what Foundation's decode attempt reports.
    var writes: [[String: Any]] = []
    for encoding in [detected.encoding, .windowsCP1252, .utf8, .utf16, .utf32] {
        var writeConfiguration = detected
        writeConfiguration.encoding = encoding
        var write: [String: Any] = [
            "encoding": try encodingID(encoding),
            "configuration": try configurationObject(writeConfiguration),
        ]
        if String(data: data, encoding: encoding) == nil {
            write["skip"] = "cannotDecodeData"
        } else {
            let writer = CSVWriter(
                table: table,
                header: nil,
                configuration: writeConfiguration,
                includeUTF8BOM: hadUTF8BOM
            )
            do {
                write["base64"] = try writer.data().base64EncodedString()
            } catch {
                write["skip"] = skipName(for: error)
            }
        }
        writes.append(write)
    }

    return [
        "name": name,
        "detected": try configurationObject(detected),
        "hadUTF8BOM": hadUTF8BOM,
        "rows": rows,
        "writes": writes,
    ]
}

func sortCases() -> [[String: Any]] {
    // Covers numbers, case pairs, natural-order text, kana and CJK, composed
    // and decomposed diacritics, and blank cells.
    let corpus = [
        "2", "10", "001.5", "1,5",
        "a", "A", "b", "B", "z", "Z",
        "あ", "ァ", "あー", "アー", "ア", "か", "カ",
        "中国", "日本語", "日本",
        "item2", "Äpfel", "Apfel", "côte", "cote", "Straße", "Strasse",
        "é", "e\u{301}", "Über", "uber",
        "apple", "Apple", "",
        " ", "\t", "  ",
        "-3", "item10", "1e2",
    ]
    precondition(corpus.count == 40, "the golden sort corpus must have 40 cells")

    let table = CSVTable(rows: corpus.map { [$0] }, headers: nil)
    var cases: [[String: Any]] = []
    for mark in [".", ","] {
        for (direction, name) in [
            (CSVSortDirection.ascending, "ascending"),
            (CSVSortDirection.descending, "descending"),
        ] {
            let order = table.rowOrder(sortedByColumn: 0, direction: direction, decimalMark: mark)
            cases.append([
                "name": "corpus-\(mark == "." ? "dot" : "comma")-\(name)",
                "decimalMark": mark,
                "direction": name,
                "cells": corpus,
                "order": order,
            ])
        }
    }
    return cases
}

func filterCases() throws -> [[String: Any]] {
    let rows = [
        ["Alice", "5", "apple"],
        ["bob", "10", "Banana"],
        ["Carol", "", "cherry"],
        ["dave", "2.5", "date"],
        ["Éva", "7", "éclair"],
        ["Zoé", "8", "e\u{301}clair"],
    ]
    let table = CSVTable(rows: rows, headers: nil)

    func criterion(
        _ op: FilterOperator, column: Int, value: String, enabled: Bool = true
    ) throws -> FilterCriterion {
        let columnID = table.columns.indices.contains(column) ? table.columns[column].id : nil
        return FilterCriterion(
            isEnabled: enabled,
            columnID: columnID,
            op: op,
            value: value
        )
    }

    let cases: [(String, [FilterCriterion])] = [
        ("equals", [try criterion(.equals, column: 1, value: "5")]),
        ("notEquals", [try criterion(.notEquals, column: 1, value: "5")]),
        ("contains", [try criterion(.contains, column: 0, value: "a")]),
        ("notContains", [try criterion(.notContains, column: 0, value: "a")]),
        ("beginsWith", [try criterion(.beginsWith, column: 0, value: "a")]),
        ("endsWith", [try criterion(.endsWith, column: 0, value: "e")]),
        ("greaterThan", [try criterion(.greaterThan, column: 1, value: "3")]),
        ("lessThan", [try criterion(.lessThan, column: 1, value: "5")]),
        ("greaterOrEqual", [try criterion(.greaterOrEqual, column: 1, value: "5")]),
        ("lessOrEqual", [try criterion(.lessOrEqual, column: 1, value: "5")]),
        ("isEmpty", [try criterion(.isEmpty, column: 1, value: "")]),
        ("isNotEmpty", [try criterion(.isNotEmpty, column: 1, value: "")]),
        ("matchesRegex", [try criterion(.matchesRegex, column: 0, value: "^[ab]")]),
        ("disabled", [try criterion(.contains, column: 0, value: "a", enabled: false)]),
        ("emptyValue", [try criterion(.contains, column: 0, value: "")]),
        ("unknownColumn", [try criterion(.equals, column: -1, value: "x")]),
        ("invalidRegex", [try criterion(.matchesRegex, column: 0, value: "(")]),
        ("and", [
            try criterion(.contains, column: 0, value: "a"),
            try criterion(.greaterThan, column: 1, value: "3"),
        ]),
        ("nfcContains", [try criterion(.contains, column: 2, value: "é")]),
        ("nfdContains", [try criterion(.contains, column: 2, value: "e\u{301}")]),
        ("nfcEquals", [try criterion(.equals, column: 2, value: "éclair")]),
        ("nfdEquals", [try criterion(.equals, column: 2, value: "e\u{301}clair")]),
        ("nfcBeginsWith", [try criterion(.beginsWith, column: 2, value: "éclair")]),
        ("nfdEndsWith", [try criterion(.endsWith, column: 2, value: "e\u{301}clair")]),
    ]

    return cases.map { name, criteria in
        let matching = RowFilter(criteria: criteria).matchingRows(in: table, decimalMark: ".")
        return [
            "name": name,
            "rows": rows,
            "criteria": criteria.map { criterion in
                [
                    "op": criterion.op.rawValue,
                    "column": table.columns.firstIndex {
                        $0.id == criterion.columnID
                    } ?? -1,
                    "value": criterion.value,
                    "enabled": criterion.isEnabled,
                ] as [String: Any]
            },
            "matching": matching,
        ]
    }
}

func clipboardSection() -> [String: Any] {
    let parseInputs = [
        "a\tb\n",
        "a\tb\r\nc\td\r\n",
        "a\tb\rc\td",
        "a\n\nb\n",
        "a\r\n\r\nb",
        "\n",
        "",
        "a\t\tb\n\n",
        "\t\n",
    ]
    let parse = parseInputs.map { input in
        ["input": input, "rows": CSVClipboard.parseTSV(input)] as [String: Any]
    }

    let decimalSeparator = Locale.current.decimalSeparator ?? ""
    let makeInputs: [(rows: [[String]], decimalMark: String)] = [
        ([["1.5", "a\tb"]], "."),
        ([["a\nb", "c\r\nd", "e\rf"]], "."),
        ([["1,5", "2"]], ","),
        ([["-3.25", "1e2"]], "."),
        ([["", "x"]], "."),
    ]
    let make = makeInputs.map { rows, decimalMark in
        [
            "rows": rows,
            "decimalMark": decimalMark,
            "separator": decimalSeparator,
            "output": CSVClipboard.makeTSV(
                rows: rows,
                numericDecimalMark: decimalMark,
                locale: Locale.current
            ),
        ] as [String: Any]
    }

    return ["parse": parse, "make": make]
}

func exportConfiguration(
    encoding: String.Encoding = .utf8,
    separator: String = ",",
    quote: String = "\"",
    escape: String = "\"",
    decimalMark: String = ".",
    header: Bool = false
) -> CSVConfiguration {
    CSVConfiguration(
        encoding: encoding,
        columnSeparator: separator,
        quoteCharacter: quote,
        escapeCharacter: escape,
        decimalMark: decimalMark,
        firstRowAsHeader: header
    )
}

func exportCases() throws -> [[String: Any]] {
    let defaultDocument = exportConfiguration()

    let cases: [(name: String, headers: [String]?, rows: [[String]],
                 document: CSVConfiguration, export: CSVConfiguration)] = [
        (
            "documentHeader",
            ["name", "score"],
            [["Alice", "1.5"], ["Bob", "2.5"]],
            defaultDocument,
            exportConfiguration(separator: ";", decimalMark: ",", header: true)
        ),
        (
            "generatedHeader",
            nil,
            [["1.5", "x"], ["2.5", "y"]],
            defaultDocument,
            exportConfiguration(header: true)
        ),
        (
            "headerOff",
            ["name", "score"],
            [["1", "2"]],
            defaultDocument,
            exportConfiguration(header: false)
        ),
        (
            "markConversion",
            nil,
            [["1,5", "x"]],
            exportConfiguration(decimalMark: ","),
            exportConfiguration(decimalMark: ".")
        ),
        (
            "ambiguityAfterConversion",
            nil,
            [["1.5", "x"]],
            defaultDocument,
            exportConfiguration(quote: "", escape: "\"", decimalMark: ",")
        ),
        (
            "unencodableText",
            nil,
            [["日本", "x"]],
            defaultDocument,
            exportConfiguration(encoding: .windowsCP1252)
        ),
        (
            "escapedFields",
            ["a\"b", "c\nd"],
            [["x\"y", "z\nw"]],
            defaultDocument,
            exportConfiguration()
        ),
        (
            "utf16Export",
            nil,
            [["1.5", "x"]],
            defaultDocument,
            exportConfiguration(encoding: .utf16)
        ),
    ]

    return try cases.map { name, headers, rows, document, export in
        let table = CSVTable(rows: rows, headers: headers)
        let issue = CSVExport.preflightIssue(
            table: table,
            documentConfiguration: document,
            exportConfiguration: export
        )
        let preflightValue: Any = if let issue {
            issue == .ambiguousOutput ? "ambiguousOutput" : "unencodableText"
        } else {
            NSNull()
        }
        var object: [String: Any] = [
            "name": name,
            "headers": headers as Any? ?? NSNull(),
            "rows": rows,
            "document": try configurationObject(document),
            "export": try configurationObject(export),
            "preflight": preflightValue,
        ]
        if issue == nil {
            object["base64"] = try CSVExport.data(
                table: table,
                documentConfiguration: document,
                exportConfiguration: export
            ).base64EncodedString()
        } else {
            object["base64"] = NSNull()
        }
        return object
    }
}

func generate(fixturesRoot: URL) throws -> [String: Any] {
    let names = try fixtureNames(in: fixturesRoot)
    return [
        "locale": Locale.current.identifier,
        "encodings": SupportedEncodings.all.map(\.name),
        "fixtures": try names.map { try fixtureObject(name: $0, fixturesRoot: fixturesRoot) },
        "sort": sortCases(),
        "filters": try filterCases(),
        "clipboard": clipboardSection(),
        "export": try exportCases(),
    ]
}

do {
    let arguments = CommandLine.arguments
    guard arguments.count >= 3 else {
        throw GoldenToolError.usage
    }

    let fixturesRoot = URL(fileURLWithPath: arguments[1], isDirectory: true)
    let outputURL = URL(fileURLWithPath: arguments[2])
    let document = try generate(fixturesRoot: fixturesRoot)
    let data = try JSONSerialization.data(
        withJSONObject: document,
        options: [.sortedKeys, .prettyPrinted, .withoutEscapingSlashes]
    )
    try data.write(to: outputURL)
} catch {
    FileHandle.standardError.write(Data("swift-golden: \(error)\n".utf8))
    exit(1)
}
