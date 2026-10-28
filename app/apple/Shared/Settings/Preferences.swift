import Foundation
import TabToolKit

/// The user's settings: their storage keys, defaults, and readers that turn
/// stored values into the types the rest of the app uses.
///
/// Readers fall back to the default for a missing, mistyped, or unknown value,
/// so a stale or hand-edited preference can never reach the document or grid.
enum Preferences {
    /// The key strings are persisted data and must not change.
    enum Key {
        static let showFormatBar = "showFormatBar"
        static let appLanguage = "appLanguage"
        static let newDocumentEncoding = "newDocumentEncoding"
        static let newDocumentSeparator = "newDocumentSeparator"
        static let newDocumentQuoteStyle = "newDocumentQuoteStyle"
        static let newDocumentDecimalMark = "newDocumentDecimalMark"
        static let newDocumentFirstRowAsHeader = "newDocumentFirstRowAsHeader"
        static let gridAlternatingRows = "gridAlternatingRows"
        static let gridRowHeight = "gridRowHeight"
        static let largeFileThresholdMiB = "largeFileThresholdMiB"
    }

    enum Default {
        static let showFormatBar = true
        static let appLanguage = AppLanguage.system
        static let encodingID = Int(String.Encoding.utf8.rawValue)
        static let separator = SeparatorOption.comma
        static let quoteStyle = QuoteStyle.doubled
        static let decimalMark = DecimalOption.dot
        static let firstRowAsHeader = false
        static let alternatingRows = true
        static let rowHeight = 24.0
        static let largeFileThresholdMiB = 100
    }

    /// The row heights, in points, offered for the grid.
    static let rowHeights: [Double] = [20, 24, 32]

    /// The large file thresholds, in MiB, offered in the settings.
    static let largeFileThresholdsMiB: [Int] = [25, 50, 100, 250, 500, 1024]

    /// The format a new document starts with.
    static func newDocumentConfiguration(_ defaults: UserDefaults = .standard) -> CSVConfiguration {
        let storedEncoding = defaults.integer(forKey: Key.newDocumentEncoding)
        let encoding = SupportedEncodings.all.first { Int($0.id) == storedEncoding }?.encoding ?? .utf8
        let separator = option(Key.newDocumentSeparator, in: defaults, default: Default.separator)
        let quoteStyle = option(Key.newDocumentQuoteStyle, in: defaults, default: Default.quoteStyle)
        let decimalMark = option(Key.newDocumentDecimalMark, in: defaults, default: Default.decimalMark)
        return CSVConfiguration(
            encoding: encoding,
            columnSeparator: separator.rawValue,
            quoteCharacter: quoteStyle.quoteCharacter,
            escapeCharacter: quoteStyle.escapeCharacter,
            decimalMark: decimalMark.rawValue,
            firstRowAsHeader: defaults.object(forKey: Key.newDocumentFirstRowAsHeader) as? Bool
                ?? Default.firstRowAsHeader
        )
    }

    static func gridAlternatingRows(_ defaults: UserDefaults = .standard) -> Bool {
        defaults.object(forKey: Key.gridAlternatingRows) as? Bool ?? Default.alternatingRows
    }

    static func gridRowHeight(_ defaults: UserDefaults = .standard) -> Double {
        let stored = defaults.double(forKey: Key.gridRowHeight)
        return rowHeights.contains(stored) ? stored : Default.rowHeight
    }

    /// The file size from which a document opens in large file mode.
    static func largeFileThresholdBytes(_ defaults: UserDefaults = .standard) -> Int64 {
        let stored = defaults.integer(forKey: Key.largeFileThresholdMiB)
        let mebibytes = largeFileThresholdsMiB.contains(stored) ? stored : Default.largeFileThresholdMiB
        return Int64(mebibytes) * 1_048_576
    }

    private static func option<Option: RawRepresentable>(
        _ key: String,
        in defaults: UserDefaults,
        default fallback: Option
    ) -> Option where Option.RawValue == String {
        defaults.string(forKey: key).flatMap(Option.init(rawValue:)) ?? fallback
    }
}
