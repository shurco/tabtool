import Foundation
import Testing
@testable import TabToolKit

@Suite struct LargeCSVGeneratorTests {
    private func generate(_ name: String, in dir: TempDirectory, bytes: Int = 1 << 20,
                          quoted: Bool = true, multiline: Int? = nil, crlf: Bool = false,
                          bom: Bool = false, seed: UInt64 = 1) throws -> (url: URL, rows: Int, size: Int) {
        let url = dir.url.appendingPathComponent(name)
        let result = try generateLargeCSV(at: url, targetBytes: bytes, columns: 5, quoted: quoted,
                                          multilineEvery: multiline, crlf: crlf, bom: bom, seed: seed)
        return (url, result.rows, result.bytes)
    }

    @Test func sameSeedGivesIdenticalBytes() throws {
        let dir = TempDirectory()
        let a = try generate("a.csv", in: dir, multiline: 7)
        let b = try generate("b.csv", in: dir, multiline: 7)
        let other = try generate("c.csv", in: dir, multiline: 7, seed: 2)
        #expect(try Data(contentsOf: a.url) == Data(contentsOf: b.url))
        #expect(try Data(contentsOf: a.url) != Data(contentsOf: other.url))
        #expect(a.size >= 1 << 20)
    }

    @Test func crlfHasNoBareLineFeedOutsideMultilineFields() throws {
        let dir = TempDirectory()
        let plain = try generate("p.csv", in: dir, crlf: true)
        let data = try Data(contentsOf: plain.url)
        #expect(data.filter { $0 == 10 }.count == plain.rows)
        #expect(data.filter { $0 == 13 }.count == plain.rows)

        let multi = try generate("m.csv", in: dir, multiline: 10, crlf: true)
        let bytes = Array(try Data(contentsOf: multi.url))
        let bare = bytes.indices.filter { bytes[$0] == 10 && bytes[$0 - 1] != 13 }
        #expect(bare.count == (multi.rows - 1) / 10)
    }

    @Test func reportedRowsMatchParser() throws {
        let dir = TempDirectory()
        let file = try generate("r.csv", in: dir, bytes: 2 << 20, multiline: 5)
        var parser = try CSVParser(data: Data(contentsOf: file.url), configuration: .init())
        #expect(try parser.readAll().count == file.rows)
    }

    @Test func bomPrefixIsWritten() throws {
        let dir = TempDirectory()
        let file = try generate("b.csv", in: dir, bytes: 1024, bom: true)
        #expect(try Data(contentsOf: file.url).prefix(3) == Data([0xEF, 0xBB, 0xBF]))
        var parser = try CSVParser(data: Data(contentsOf: file.url), configuration: .init())
        #expect(parser.hadUTF8BOM)
        #expect(try parser.readAll().count == file.rows)
    }
}
