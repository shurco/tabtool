import Foundation

/// Status-bar wording for large file mode.
enum LargeStatusText {
    static func indexing(fraction: Double, rows: Int) -> String {
        let percent = Int(min(max(fraction, 0), 1) * 100)
        return String(localized: "Indexing… \(percent)% · \(rows.formatted()) rows so far")
    }
}
