import Foundation
import Testing
@testable import TabToolKit

@Test func supportedEncodingsMatchDisplayOrder() {
    #expect(SupportedEncodings.all.count == 18)
    #expect(SupportedEncodings.all[0].name == "Unicode (UTF-8)")
    #expect(SupportedEncodings.all[0].encoding == .utf8)
    #expect(SupportedEncodings.all.contains { $0.name == "Chinese (GBK)" })
    #expect(SupportedEncodings.all.contains { $0.encoding == .windowsCP1252 })
}

@Test func supportedEncodingsMatchExpectedTable() {
    let expected: [(name: String, rawValue: UInt)] = [
        ("Unicode (UTF-8)", 0x4),
        ("Western (Mac OS Roman)", 0x1e),
        ("Western (Windows Latin 1)", 0xc),
        ("Chinese (GBK)", 0x80000632),
        ("Central European (ISO Latin 2)", 0x9),
        ("Central European (Windows Latin 2)", 0xf),
        ("Cyrillic (Windows)", 0xb),
        ("Greek (Windows)", 0xd),
        ("Turkish (Windows)", 0xe),
        ("Japanese (EUC)", 0x3),
        ("Japanese (Shift_JIS)", 0x8),
        ("Japanese (ISO 2022-JP)", 0x15),
        ("Unicode (UTF-16)", 0xa),
        ("Unicode (UTF-16, Big Endian)", 0x90000100),
        ("Unicode (UTF-16, Little Endian)", 0x94000100),
        ("Unicode (UTF-32)", 0x8c000100),
        ("Unicode (UTF-32, Big Endian)", 0x98000100),
        ("Unicode (UTF-32, Little Endian)", 0x9c000100)
    ]

    #expect(SupportedEncodings.all.count == expected.count)

    for (actual, expected) in zip(SupportedEncodings.all, expected) {
        #expect(actual.name == expected.name)
        #expect(actual.encoding.rawValue == expected.rawValue)
        #expect(actual.id == expected.rawValue)
    }
}

@Test func supportedEncodingIdentifiersAreUnique() {
    let ids = SupportedEncodings.all.map(\.id)
    #expect(Set(ids).count == ids.count)
}
