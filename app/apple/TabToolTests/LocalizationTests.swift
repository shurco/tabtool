import Foundation
import Testing

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
}
