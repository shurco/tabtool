import CoreFoundation
import Foundation

/// An encoding offered in the format bar, with a stable identifier for the UI.
public struct SupportedEncoding: Identifiable, Sendable, Hashable {
    public let id: UInt
    public let name: String
    public let encoding: String.Encoding
}

/// The encoding choices, in display order.
public enum SupportedEncodings {
    /// GBK is not exposed as a `String.Encoding` convenience value, so the
    /// Core Foundation encoding constant is converted to its Foundation value.
    private static let gbk = String.Encoding(
        rawValue: CFStringConvertEncodingToNSStringEncoding(
            CFStringEncoding(CFStringEncodings.GB_18030_2000.rawValue)
        )
    )

    public static let all: [SupportedEncoding] = [
        make("Unicode (UTF-8)", .utf8),
        make("Western (Mac OS Roman)", .macOSRoman),
        make("Western (Windows Latin 1)", .windowsCP1252),
        make("Chinese (GBK)", gbk),
        make("Central European (ISO Latin 2)", .isoLatin2),
        make("Central European (Windows Latin 2)", .windowsCP1250),
        make("Cyrillic (Windows)", .windowsCP1251),
        make("Greek (Windows)", .windowsCP1253),
        make("Turkish (Windows)", .windowsCP1254),
        make("Japanese (EUC)", .japaneseEUC),
        make("Japanese (Shift_JIS)", .shiftJIS),
        make("Japanese (ISO 2022-JP)", .iso2022JP),
        make("Unicode (UTF-16)", .utf16),
        make("Unicode (UTF-16, Big Endian)", .utf16BigEndian),
        make("Unicode (UTF-16, Little Endian)", .utf16LittleEndian),
        make("Unicode (UTF-32)", .utf32),
        make("Unicode (UTF-32, Big Endian)", .utf32BigEndian),
        make("Unicode (UTF-32, Little Endian)", .utf32LittleEndian)
    ]

    private static func make(_ name: String, _ encoding: String.Encoding) -> SupportedEncoding {
        SupportedEncoding(id: encoding.rawValue, name: name, encoding: encoding)
    }
}
