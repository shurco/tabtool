import Foundation

/// A fixture file that is missing from the test resources.
struct FixtureNotFound: Error, CustomStringConvertible {
    let name: String

    var description: String { "Missing fixture: \(name).csv" }
}

/// Loads a CSV fixture by name, searching the shared fixture directories.
func fixture(_ name: String) throws -> Data {
    for folder in ["Reading", "Heuristic", "Performance"] {
        let subdirectory = "Fixtures/\(folder)"
        if let url = Bundle.module.url(forResource: name,
                                       withExtension: "csv",
                                       subdirectory: subdirectory) {
            return try Data(contentsOf: url)
        }
    }
    throw FixtureNotFound(name: name)
}
