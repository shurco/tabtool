import Foundation

/// Status-bar wording for large file mode.
enum LargeStatusText {
    static func indexing(fraction: Double, rows: Int) -> String {
        String(localized: "Indexing… \(percent(fraction))% · \(rows.formatted()) rows so far")
    }

    static func saving(fraction: Double) -> String {
        String(localized: "Saving… \(percent(fraction))%")
    }

    private static func percent(_ fraction: Double) -> Int {
        Int(min(max(fraction, 0), 1) * 100)
    }
}
