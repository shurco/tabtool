import Foundation

/// A uniquely named `UserDefaults` suite removed with the test, so tests never touch `UserDefaults.standard`.
final class TestDefaults {
    let defaults: UserDefaults
    private let suiteName = "tabtool-tests-\(UUID().uuidString)"

    /// Values stored in the suite itself; `object(forKey:)` would fall through to the global domain.
    var persistentValues: [String: Any] { defaults.persistentDomain(forName: suiteName) ?? [:] }

    init() { defaults = UserDefaults(suiteName: suiteName)! }
    deinit { defaults.removePersistentDomain(forName: suiteName) }
}
