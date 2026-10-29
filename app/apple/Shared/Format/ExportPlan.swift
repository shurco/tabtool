import Foundation
import TabToolKit
import UniformTypeIdentifiers

/// The file mapping and validation rules for an export.
///
/// These rules are independent of AppKit so they can be unit tested. The
/// export always follows the export configuration: a tab separator writes
/// TSV, every other separator writes CSV.
enum ExportPlan {
    /// The file name stem offered by the panel: the document name plus
    /// `-export`, or `Untitled-export` when the document has no file yet.
    static func baseName(forDocumentAt url: URL?) -> String {
        let name = url?.deletingPathExtension().lastPathComponent ?? ""
        return (name.isEmpty ? "Untitled" : name) + "-export"
    }

    /// The complete file name for the seeded configuration.
    static func filename(baseName: String, configuration: CSVConfiguration) -> String {
        "\(baseName).\(fileExtension(for: configuration))"
    }

    /// The file extension an export writes.
    static func fileExtension(for configuration: CSVConfiguration) -> String {
        configuration.columnSeparator == "\t" ? "tsv" : "csv"
    }

    /// The content type shown by the save panel.
    static func contentType(for configuration: CSVConfiguration) -> UTType {
        fileExtension(for: configuration) == "tsv"
            ? .tabSeparatedText
            : .commaSeparatedText
    }

    /// Replaces the extension of a file name while keeping its stem.
    ///
    /// This runs before the panel validates the name, so the panel owns the
    /// final extension and any replace confirmation.
    static func renaming(_ filename: String, configuration: CSVConfiguration) -> String {
        let stem = (filename as NSString).deletingPathExtension
        guard !stem.isEmpty else { return filename }
        return "\(stem).\(fileExtension(for: configuration))"
    }

    /// Writes the exported bytes to `url` atomically.
    ///
    /// Validation happens before any file access, so an unencodable or
    /// ambiguous export never leaves a partial file behind.
    static func write(table: CSVTable,
                      documentConfiguration: CSVConfiguration,
                      configuration: CSVConfiguration,
                      to url: URL,
                      allowAmbiguousOutput: Bool = false) throws {
        let data = try CSVExport.data(
            table: table,
            documentConfiguration: documentConfiguration,
            exportConfiguration: configuration,
            allowAmbiguousOutput: allowAmbiguousOutput
        )
        try data.write(to: url, options: .atomic)
    }

    /// What the UI does for the output `table` will produce.
    ///
    /// The decision comes from the same converted table and header the writer
    /// uses, because decimal conversion can create or remove separators inside
    /// numeric cells.
    static func decision(table: CSVTable,
                         documentConfiguration: CSVConfiguration,
                         exportConfiguration: CSVConfiguration) -> ExportDecision {
        decision(for: CSVExport.preflightIssue(
            table: table,
            documentConfiguration: documentConfiguration,
            exportConfiguration: exportConfiguration
        ))
    }

    /// What the UI does for a validation result.
    static func decision(for issue: CSVWriteIssue?) -> ExportDecision {
        switch issue {
        case nil:
            return .proceed
        case .ambiguousOutput:
            return .confirmAmbiguous
        case .unencodableText:
            return .blockedUnencodable
        }
    }
}

/// Whether export can write, needs confirmation, or cannot write at all.
enum ExportDecision: Equatable {
    case proceed
    case confirmAmbiguous
    case blockedUnencodable
}
