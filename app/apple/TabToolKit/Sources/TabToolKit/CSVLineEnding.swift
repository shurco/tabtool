/// The row terminator style of a CSV file and whether the file ends with one.
public struct CSVLineEnding: Sendable, Equatable {
    public enum Terminator: String, Sendable {
        case lf = "\n"
        case crlf = "\r\n"
        case cr = "\r"
    }

    public var terminator: Terminator
    public var endsWithTerminator: Bool

    public init(terminator: Terminator, endsWithTerminator: Bool) {
        self.terminator = terminator
        self.endsWithTerminator = endsWithTerminator
    }

    /// Line feed without a trailing terminator.
    public static let `default` = CSVLineEnding(terminator: .lf, endsWithTerminator: false)
}
