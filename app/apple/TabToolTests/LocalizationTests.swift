import Foundation
import Testing
@testable import TabTool

/// The texts of the large file mode are in `Localizable.xcstrings`. The test
/// host is the app, so its compiled string table answers.
@Suite struct LocalizationTests {
    @Test(arguments: [
        "Large file mode. You can browse and edit cells. Sorting, adding, deleting or moving rows and columns, renaming columns, changing the decimal mark and exporting are not available for files this size.",
        "Indexing… %lld%% · %@ rows so far",
        "Saving… %lld%%",
        "Cancel",
        "The file changed on disk. Reload to continue.",
        "New rows were appended. Reload to see them.",
        "Reload",
        "Not available in large file mode.",
        "This encoding cannot be opened in large file mode.",
        "Convert the file to UTF-8 and open it again.",
        "Large file mode needs a single-character ASCII separator, quote and escape.",
        "Choose a separator, quote and escape that are single ASCII characters.",
        "Edit Not Applied",
        "This row cannot be edited right now.",
        "Wait for indexing to finish, or reload the file if it changed on disk.",
        "Too many cells selected to copy (limit 1,000,000).",
        "Do you want to save the changes made to “%@”?",
        "Your changes will be lost if you don’t save them.",
        "Save",
        "Don’t Save",
        "The file could not be saved",
        "Indexing is not finished. Save again when it completes.",
        "The file changed on disk since it was opened. Reload it before saving.",
        "There is not enough free disk space to save this file.",
        "Saving a large file needs free space of about twice its size.",
    ])
    func largeFileTextIsInTheCatalog(_ key: String) {
        let missing = "\u{0}missing"
        #expect(Bundle.main.localizedString(forKey: key, value: missing, table: nil) != missing)
    }

    /// Every English key has a translation that keeps the key's format specifiers.
    @Test(arguments: AppLanguage.allCases.filter { $0 != .system && $0 != .en })
    func everyKeyIsTranslated(_ language: AppLanguage) throws {
        let english = stringTable(for: .en)
        let translated = stringTable(for: language)
        try #require(!english.isEmpty, "the English table is missing from the app bundle")
        var problems: [String] = []
        for key in english.keys.sorted() {
            guard let translation = translated[key], !translation.isEmpty else {
                problems.append("missing: \(key)")
                continue
            }
            if !keepsFormatSpecifiers(translation, of: key) {
                problems.append("format specifiers differ: \(key)")
            }
        }
        #expect(problems.isEmpty, "\(language.rawValue): \(problems.joined(separator: "\n"))")
    }

    /// The strings looked up at runtime, not through a SwiftUI literal, must be in the catalog.
    @Test func runtimeLookupsResolveInFrench() throws {
        let french = try frenchBundle()
        #expect(french.localized("Decimal") == "Décimal")
        #expect(french.localized("Edit Cell") == "Modifier la cellule")
        #expect(FormatDisplay.separatorName(for: ",", in: french) == "Virgule")
        #expect(FormatDisplay.encodingName(for: .macOSRoman, in: french) == "Occidental (Mac OS Roman)")
    }

    @Test(arguments: [
        "Paste Data", "Edit Cell", "Add Row", "Delete Rows", "Move Rows", "Add Column", "Delete Columns",
        "Move Column", "Move Columns", "Change Format", "Toggle Header", "Sort Rows", "Remove Sort",
        "Rename Column", "Revert to Saved",
    ])
    func undoActionNamesAreTranslated(_ name: String) throws {
        #expect(try frenchBundle().localized(name) != name)
    }

    @Test func formatSpecifiersIgnorePositionalIndexes() {
        #expect(keepsFormatSpecifiers("%1$@ %2$lld", of: "%lld %@"))
        #expect(keepsFormatSpecifiers("%2$lld %1$@", of: "%@ %lld"))
        #expect(!keepsFormatSpecifiers("%1$@", of: "%lld"))
    }

    @Test func plainFormatSpecifiersKeepTheirOrder() {
        #expect(keepsFormatSpecifiers("%lld%% · %@", of: "%lld%% · %@"))
        #expect(!keepsFormatSpecifiers("%lld %@", of: "%@ %lld"))
        #expect(!keepsFormatSpecifiers("%@", of: "%lld"))
    }
}

/// The app bundle's French resources, to check lookups that take a bundle.
private func frenchBundle() throws -> Bundle {
    let path = try #require(Bundle.main.path(forResource: "fr", ofType: "lproj"))
    return try #require(Bundle(path: path))
}

/// The compiled `Localizable.strings` of a language, read from the app bundle.
/// A language without any translated key has no table, so it reads as empty.
private func stringTable(for language: AppLanguage) -> [String: String] {
    Bundle.main.url(
        forResource: "Localizable", withExtension: "strings", subdirectory: nil, localization: language.rawValue
    )
    .flatMap { NSDictionary(contentsOf: $0) as? [String: String] } ?? [:]
}

/// Whether a translation uses the same format specifier types as its key.
///
/// Without positional indexes the specifiers keep the key's order, because the arguments are
/// substituted in that order. With positional indexes (`%2$lld`) the order may change, so only
/// the types are compared.
private func keepsFormatSpecifiers(_ translation: String, of key: String) -> Bool {
    let specifier = /%(\d+\$)?([lh]*[@dDfsSuUxXcCp]|%)/
    let found = translation.matches(of: specifier)
    let types = found.map { "%" + $0.output.2 }
    let keyTypes = key.matches(of: specifier).map { "%" + $0.output.2 }
    return found.contains { $0.output.1 != nil } ? types.sorted() == keyTypes.sorted() : types == keyTypes
}
