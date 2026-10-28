import Foundation

/// A `UserDefaults` suite whose plist is deleted with the test, so tests never touch `UserDefaults.standard`.
///
/// The suite name is a path in the temporary directory. A named suite would keep an empty plist in the
/// Preferences folder, which cfprefsd rewrites after the test even when the file is deleted.
final class TestDefaults {
    let defaults: UserDefaults
    private let suiteName = FileManager.default.temporaryDirectory.appending(path: "tabtool-tests-\(UUID().uuidString)").path

    /// Values stored in the suite itself; `object(forKey:)` would fall through to the global domain.
    var persistentValues: [String: Any] { defaults.persistentDomain(forName: suiteName) ?? [:] }

    init() { defaults = UserDefaults(suiteName: suiteName)! }

    deinit {
        defaults.removePersistentDomain(forName: suiteName)
        try? FileManager.default.removeItem(atPath: "\(suiteName).plist")
    }
}
