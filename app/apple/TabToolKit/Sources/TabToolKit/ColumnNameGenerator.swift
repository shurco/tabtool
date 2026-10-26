/// Generates spreadsheet-style column names.
public enum ColumnNameGenerator {
    private static let alphabet = Array("ABCDEFGHIJKLMNOPQRSTUVWXYZ")

    /// Returns the name for a zero-based column index: A…Z, AA…AZ, BA…, and
    /// so on. A negative index has no name and returns an empty string.
    public static func name(for index: Int) -> String {
        guard index >= 0 else { return "" }

        var value = index
        var name = ""
        repeat {
            let digit = value % alphabet.count
            name = String(alphabet[digit]) + name
            value = value / alphabet.count - 1
        } while value >= 0
        return name
    }
}
