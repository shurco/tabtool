import TabToolKit

/// The operation a format-bar edit needs, derived from the configuration diff.
enum FormatChange: Equatable {
    /// The configuration did not change.
    case none
    /// Convert numeric cells in memory from the old mark to the new one.
    case decimalMark(String)
    /// Move the first row across the header boundary without re-reading.
    case header(Bool)
    /// Re-read the file (or apply the format to an untitled table).
    case reparse(CSVConfiguration)
}

/// Maps a configuration edit to the smallest operation that applies it.
///
/// Parsing fields win over the in-memory transforms: a new encoding,
/// separator, quote character, or escape character changes how the bytes are
/// read, so it must re-read them. The format bar changes one control at a
/// time; the ordering below only matters when a caller substitutes several
/// fields at once.
enum FormatChangePlanner {
    static func plan(from old: CSVConfiguration,
                     to new: CSVConfiguration) -> FormatChange {
        guard old != new else { return .none }

        if old.encoding != new.encoding
            || old.columnSeparator != new.columnSeparator
            || old.quoteCharacter != new.quoteCharacter
            || old.escapeCharacter != new.escapeCharacter {
            return .reparse(new)
        }
        if old.decimalMark != new.decimalMark {
            return .decimalMark(new.decimalMark)
        }
        if old.firstRowAsHeader != new.firstRowAsHeader {
            return .header(new.firstRowAsHeader)
        }
        return .none
    }
}
