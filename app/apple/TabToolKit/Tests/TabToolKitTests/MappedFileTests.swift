import Foundation
import Testing
@testable import TabToolKit

/// A scratch directory removed when the value goes away.
final class TempDirectory {
    let url: URL

    init() {
        url = FileManager.default.temporaryDirectory
            .appendingPathComponent("tabtool-\(UUID().uuidString)", isDirectory: true)
        try? FileManager.default.createDirectory(at: url, withIntermediateDirectories: true)
    }

    deinit { try? FileManager.default.removeItem(at: url) }

    func file(_ name: String, _ bytes: [UInt8]) throws -> URL {
        let fileURL = url.appendingPathComponent(name)
        try Data(bytes).write(to: fileURL)
        return fileURL
    }
}

private func bytes(_ file: MappedFile, _ range: Range<Int>) -> [UInt8] {
    file.withBytes(range) { Array($0) }
}

@Suite struct MappedFileTests {
    @Test func mapsContent() throws {
        let dir = TempDirectory()
        let file = try MappedFile(url: dir.file("a.csv", Array("hello,world\n".utf8)))
        #expect(file.size == 12)
        #expect(bytes(file, 0..<12) == Array("hello,world\n".utf8))
        #expect(bytes(file, 6..<11) == Array("world".utf8))
    }

    @Test func emptyFileHasZeroSize() throws {
        let dir = TempDirectory()
        let file = try MappedFile(url: dir.file("e.csv", []))
        #expect(file.size == 0)
        #expect(bytes(file, 0..<10).isEmpty)
        #expect(file.isStale() == .unchanged)
    }

    @Test func clampsRange() throws {
        let dir = TempDirectory()
        let file = try MappedFile(url: dir.file("a.csv", [1, 2, 3, 4]))
        #expect(bytes(file, 2..<100) == [3, 4])
        #expect(bytes(file, 50..<60).isEmpty)
    }

    @Test func readGivesTheSameBytesAsTheMapping() throws {
        let dir = TempDirectory()
        let content = (0..<20_000).map { UInt8($0 % 251) }
        let file = try MappedFile(url: dir.file("a.csv", content))
        var buffer: [UInt8] = []
        for range in [0..<20_000, 5_000..<13_001, 4_096..<8_192, 19_990..<20_500, 30_000..<31_000, 7..<7] {
            let expected = file.withBytes(range) { Array($0) }
            #expect(file.read(range, into: &buffer) { Array($0) } == expected)
        }
    }

    @Test func readOfAnEmptyFileIsEmpty() throws {
        let dir = TempDirectory()
        let file = try MappedFile(url: dir.file("e.csv", []))
        var buffer: [UInt8] = []
        #expect(file.read(0..<10, into: &buffer) { Array($0) }.isEmpty)
    }

    @Test func readStopsAtATruncatedEnd() throws {
        let dir = TempDirectory()
        let url = try dir.file("a.csv", Array(repeating: 7, count: 20_000))
        let file = try MappedFile(url: url)
        try FileHandle(forWritingTo: url).truncate(atOffset: 100)
        var buffer: [UInt8] = []
        #expect(file.read(0..<20_000, into: &buffer) { $0.count } == 100)
    }

    @Test func detectsUnchanged() throws {
        let dir = TempDirectory()
        let file = try MappedFile(url: dir.file("a.csv", [1, 2, 3]))
        #expect(file.isStale() == .unchanged)
    }

    @Test func detectsAppend() throws {
        let dir = TempDirectory()
        let url = try dir.file("a.csv", [1, 2, 3])
        let file = try MappedFile(url: url)
        let handle = try FileHandle(forWritingTo: url)
        try handle.seekToEnd()
        try handle.write(contentsOf: Data([4, 5]))
        try handle.close()
        #expect(file.isStale() == .appended)
    }

    @Test func detectsTruncation() throws {
        let dir = TempDirectory()
        let url = try dir.file("a.csv", [1, 2, 3, 4])
        let file = try MappedFile(url: url)
        try FileHandle(forWritingTo: url).truncate(atOffset: 2)
        #expect(file.isStale() == .replacedOrTruncated)
        #expect(!file.isReadable)
    }

    @Test func detectsReplace() throws {
        let dir = TempDirectory()
        let url = try dir.file("a.csv", [1, 2, 3])
        let file = try MappedFile(url: url)
        let other = try dir.file("b.csv", [1, 2, 3])
        _ = try FileManager.default.replaceItemAt(url, withItemAt: other)
        #expect(file.isStale() == .replacedOrTruncated)
        // The open descriptor still holds the old file.
        #expect(file.isReadable)
    }

    @Test func missingFileThrows() {
        let url = FileManager.default.temporaryDirectory
            .appendingPathComponent("missing-\(UUID().uuidString).csv")
        #expect(throws: (any Error).self) { try MappedFile(url: url) }
    }
}
