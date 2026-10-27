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

/// A deterministic CSV with a header row: even columns hold numbers, odd
/// columns hold text, and every tenth row of a quoted file holds a multiline
/// quoted field with an embedded separator and quotes.
func makeBenchmarkCSV(rows: Int, cols: Int, quoted: Bool) -> Data {
    var generator = SeededGenerator(seed: 42)
    var text = (0..<cols).map { "col\($0)" }.joined(separator: ",") + "\n"
    for row in 0..<rows {
        let fields = (0..<cols).map { column -> String in
            if column % 2 == 0 {
                return "\(Int(generator.next() % 1_000_000)).\(column)"
            }
            if quoted && row % 10 == 0 {
                return "\"line one\nline, \"\"two\"\" \(row)\""
            }
            return "text\(generator.next() % 5000)"
        }
        text += fields.joined(separator: ",") + "\n"
    }
    return Data(text.utf8)
}
