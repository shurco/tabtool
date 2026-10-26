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

/// Loads a CSV fixture from an explicit resource subdirectory.
func fixtureData(_ name: String, subdirectory: String) throws -> Data {
    guard let url = Bundle.module.url(forResource: name,
                                      withExtension: "csv",
                                      subdirectory: subdirectory) else {
        throw FixtureNotFound(name: "\(subdirectory)/\(name)")
    }
    return try Data(contentsOf: url)
}

/// A deterministic SplitMix64 generator for repeatable random tables.
struct SeededGenerator: RandomNumberGenerator {
    private var state: UInt64

    init(seed: UInt64) {
        state = seed
    }

    mutating func next() -> UInt64 {
        state &+= 0x9E37_79B9_7F4A_7C15
        var value = state
        value = (value ^ (value >> 30)) &* 0xBF58_476D_1CE4_E5B9
        value = (value ^ (value >> 27)) &* 0x94D0_49BB_1331_11EB
        return value ^ (value >> 31)
    }
}
