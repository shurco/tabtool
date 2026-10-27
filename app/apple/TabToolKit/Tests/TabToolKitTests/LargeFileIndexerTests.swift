import Foundation
import Testing
@testable import TabToolKit

private struct Fixture {
    let dir = TempDirectory()
    let url: URL
    let file: MappedFile
    let rows: Int

    init(bytes: Int = 2 << 20, multilineEvery: Int? = 7) throws {
        url = dir.url.appendingPathComponent("big.csv")
        rows = try generateLargeCSV(at: url, targetBytes: bytes, columns: 5, quoted: true,
                                    multilineEvery: multilineEvery, crlf: false, bom: false,
                                    seed: 3).rows
        file = try MappedFile(url: url)
    }
}

private func indexer(_ fixture: Fixture) -> LargeFileIndexer {
    LargeFileIndexer(file: fixture.file, configuration: CSVConfiguration(),
                     stepBytes: 4096, progressInterval: 0)
}

@Suite struct LargeFileIndexerTests {
    @Test func finalCountMatchesGenerator() throws {
        let fixture = try Fixture()
        let indexer = indexer(fixture)
        indexer.start { _ in }
        indexer.wait()
        #expect(indexer.isFinished)
        #expect(indexer.snapshot.rowCount == fixture.rows)
        #expect(indexer.snapshot.maxFieldCount == 5)
        #expect(indexer.snapshot.scannedBytes == fixture.file.size)
    }

    @Test func matchesDirectScan() throws {
        let fixture = try Fixture()
        let indexer = indexer(fixture)
        indexer.start { _ in }
        indexer.wait()
        var direct = RowIndex()
        var scanner = RowScanner(configuration: CSVConfiguration())
        fixture.file.withBytes(0..<fixture.file.size) {
            scanner.scan($0, base: 0, into: &direct, final: true)
        }
        #expect(indexer.snapshot == direct)
    }

    @Test func partialSnapshotsAreMonotonePrefixes() throws {
        let fixture = try Fixture()
        let indexer = indexer(fixture)
        let collected = Locked<[RowIndex]>([])
        indexer.start { _ in collected.withValue { $0.append(indexer.snapshot) } }
        indexer.wait()
        let snapshots = collected.withValue { $0 }
        #expect(snapshots.count > 10)
        let final = indexer.snapshot
        for (earlier, later) in zip(snapshots, snapshots.dropFirst()) {
            #expect(earlier.rowCount <= later.rowCount)
            #expect(earlier.scannedBytes <= later.scannedBytes)
            #expect(later.checkpoints.starts(with: earlier.checkpoints))
        }
        for snapshot in snapshots {
            #expect(final.checkpoints.starts(with: snapshot.checkpoints))
        }
    }

    @Test func firstSnapshotAvailableBeforeEnd() throws {
        let fixture = try Fixture()
        let indexer = indexer(fixture)
        let first = Locked<(rows: Int, finished: Bool)?>(nil)
        indexer.start { progress in
            first.withValue { if $0 == nil { $0 = (indexer.snapshot.rowCount, progress.isFinished) } }
        }
        indexer.wait()
        let seen = try #require(first.withValue { $0 })
        #expect(seen.rows > 0)
        #expect(!seen.finished)
    }

    @Test func cancelStopsEarly() throws {
        let fixture = try Fixture()
        let indexer = indexer(fixture)
        indexer.start { _ in indexer.cancel() }
        indexer.wait()
        #expect(!indexer.isFinished)
        #expect(indexer.snapshot.scannedBytes < fixture.file.size)
    }

    @Test func emptyFileIsOneRow() throws {
        let dir = TempDirectory()
        let file = try MappedFile(url: dir.file("e.csv", []))
        let indexer = LargeFileIndexer(file: file, configuration: CSVConfiguration())
        indexer.start { _ in }
        indexer.wait()
        #expect(indexer.isFinished)
        #expect(indexer.snapshot.rowCount == 1)
    }

    @Test func endsInsideQuoteFlag() throws {
        let dir = TempDirectory()
        let file = try MappedFile(url: dir.file("q.csv", Array("a,b\n\"open,1\n2,3\n".utf8)))
        let indexer = LargeFileIndexer(file: file, configuration: CSVConfiguration())
        indexer.start { _ in }
        indexer.wait()
        #expect(indexer.snapshot.endsInsideQuote)
    }

    @Test func progressFractionReachesOne() throws {
        let fixture = try Fixture()
        let indexer = indexer(fixture)
        let last = Locked<IndexProgress?>(nil)
        indexer.start { progress in last.withValue { $0 = progress } }
        indexer.wait()
        let progress = try #require(last.withValue { $0 })
        #expect(progress.isFinished)
        #expect(progress.fraction == 1)
        #expect(progress.rowCount == fixture.rows)
    }
}
