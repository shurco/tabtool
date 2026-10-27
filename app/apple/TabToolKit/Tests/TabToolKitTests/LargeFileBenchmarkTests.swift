import Darwin
import Foundation
import Testing
@testable import TabToolKit

/// Large file mode benchmark: open, index, cell reads and streaming saves on a
/// generated file of `TABTOOL_BENCH_BYTES` bytes. Skipped unless that
/// variable is set; run it through `scripts/bench-large`, which builds in
/// release and prints the Markdown table this test writes.
@Suite(.enabled(if: ProcessInfo.processInfo.environment["TABTOOL_BENCH_BYTES"] != nil))
struct LargeFileBenchmarkTests {
    private static let clock = ContinuousClock()

    @Test func benchmark() throws {
        let target = try #require(Int(ProcessInfo.processInfo.environment["TABTOOL_BENCH_BYTES"] ?? ""))
        let dir = TempDirectory()
        let url = dir.url.appendingPathComponent("bench.csv")
        var report = ["| Metric | Value |", "|---|---|"]
        func add(_ metric: String, _ value: String) { report.append("| \(metric) | \(value) |") }

        let (generated, generation) = try Self.measure {
            try generateLargeCSV(at: url, targetBytes: target, columns: 8, quoted: true,
                                 multilineEvery: 1000, crlf: false, bom: false, seed: 42)
        }
        add("File", "\(Self.gib(generated.bytes)), \(generated.rows) rows x 8 columns (generated in \(Self.seconds(generation)))")
        add("Footprint before open", Self.mib(physFootprint()))

        // Open: map, sniff, index; the first rows show once the first block is indexed.
        let opened = Self.clock.now
        let file = try MappedFile(url: url)
        let configuration = try LargeFileSniffer.sniff(file, prefersChineseEncoding: false)
        let indexer = LargeFileIndexer(file: file, configuration: configuration)
        indexer.start { _ in }
        let source = LargeCSVSource(file: file, configuration: configuration, indexer: indexer)
        while source.rowCount < LargeFile.blockRows && !source.isIndexed { usleep(100) }
        for row in 0..<LargeFile.blockRows { _ = source.cell(row: row, column: 7) }
        add("Open to first 64 rows", Self.milliseconds(Self.clock.now - opened))
        indexer.wait()
        let indexed = Self.clock.now - opened
        let rate = Double(file.size) / Self.secondsValue(indexed) / 1e9
        add("Open to full index", "\(Self.seconds(indexed)) (\(String(format: "%.2f", rate)) GB/s)")
        add("Footprint after index", Self.mib(physFootprint()))

        var random = SeededGenerator(seed: 7)
        let jumps = (0..<1000).map { _ -> Duration in
            let row = Int(random.next() % UInt64(source.rowCount))
            return Self.clock.measure { _ = source.cell(row: row, column: 3) }
        }.sorted()
        add("Jump to a random row (1000 reads)", "p50 \(Self.milliseconds(jumps[500])), p99 \(Self.milliseconds(jumps[990]))")

        let (hits, misses) = (source.cacheHits, source.cacheMisses)
        let start = Int(random.next() % UInt64(source.rowCount - 100_000))
        let scroll = Self.clock.measure {
            for row in start..<start + 100_000 {
                for column in 0..<source.columnCount { _ = source.cell(row: row, column: column) }
            }
        }
        let hitRate = Double(source.cacheHits - hits)
            / Double(source.cacheHits - hits + source.cacheMisses - misses) * 100
        add("Scroll 100k rows x 8 cells", "\(Self.milliseconds(scroll)), cache hit rate \(String(format: "%.1f", hitRate))%")
        for _ in 0..<10_000 { _ = source.cell(row: Int(random.next() % UInt64(source.rowCount)), column: 1) }
        add("Footprint after 10k random reads", Self.mib(physFootprint()))

        for count in [0, 1, 1000] {
            let edits = try Self.edit(source, count: count, random: &random)
            let output = dir.url.appendingPathComponent("saved.csv")
            let snapshot = source.saveSnapshot()
            let ((_, elapsed), peak) = try Self.peakFootprint {
                try Self.measure { try LargeFileSaver.write(snapshot, to: output) { _ in } }
            }
            add("Save, \(count) edits", "\(Self.seconds(elapsed)), peak footprint \(Self.mib(peak))")
            try Self.verify(output, against: source, at: url, edits: edits, random: &random)
            source.overlay.removeAll()
            try FileManager.default.removeItem(at: output)
        }
        print(report.joined(separator: "\n"))
    }

    /// Sets `count` random cells and returns their texts, keyed by displayed row.
    private static func edit(_ source: LargeCSVSource, count: Int,
                             random: inout SeededGenerator) throws -> [CellKey: String] {
        var edits: [CellKey: String] = [:]
        while edits.count < count {
            let key = CellKey(row: Int(random.next() % UInt64(source.rowCount)),
                              column: Int(random.next() % UInt64(source.columnCount)))
            let text = "edited \(edits.count), \"quoted\""
            try source.setCell(text, row: key.row, column: key.column)
            edits[key] = text
        }
        return edits
    }

    /// Without edits the output must be byte-identical; with edits it must
    /// hold them and agree with the source on 10k sampled rows.
    private static func verify(_ output: URL, against source: LargeCSVSource, at url: URL,
                               edits: [CellKey: String], random: inout SeededGenerator) throws {
        let saved = try MappedFile(url: output)
        guard !edits.isEmpty else {
            let original = try MappedFile(url: url)
            #expect(saved.size == original.size)
            var (a, b) = ([UInt8](), [UInt8]())
            for offset in stride(from: 0, to: saved.size, by: 4 << 20) {
                let range = offset..<min(offset + (4 << 20), saved.size)
                let equal = saved.read(range, into: &a) { x in original.read(range, into: &b) { y in
                    x.count == y.count && memcmp(x.baseAddress!, y.baseAddress!, x.count) == 0 } }
                #expect(equal, "bytes differ in \(range)")
            }
            return
        }
        let indexer = LargeFileIndexer(file: saved, configuration: source.configuration)
        indexer.start { _ in }
        indexer.wait()
        let copy = LargeCSVSource(file: saved, configuration: source.configuration, indexer: indexer)
        #expect(copy.rowCount == source.rowCount)
        for (key, text) in edits { #expect(copy.cell(row: key.row, column: key.column) == text) }
        for _ in 0..<10_000 {
            let row = Int(random.next() % UInt64(source.rowCount))
            for column in 0..<source.columnCount {
                #expect(copy.cell(row: row, column: column) == source.cell(row: row, column: column))
            }
        }
    }

    // MARK: Measuring

    private static func measure<R>(_ body: () throws -> R) rethrows -> (R, Duration) {
        let start = clock.now
        let result = try body()
        return (result, clock.now - start)
    }

    /// Runs `body` while a background thread samples the footprint every 5 ms.
    private static func peakFootprint<R>(_ body: () throws -> R) rethrows -> (R, Int) {
        let state = Locked((peak: physFootprint(), running: true))
        let stopped = DispatchSemaphore(value: 0)
        Thread.detachNewThread {
            while state.withValue({ $0.running }) {
                let now = physFootprint()
                state.withValue { $0.peak = max($0.peak, now) }
                usleep(5_000)
            }
            stopped.signal()
        }
        let result = try body()
        state.withValue { $0.running = false }
        stopped.wait()
        return (result, state.withValue { max($0.peak, physFootprint()) })
    }

    private static func secondsValue(_ duration: Duration) -> Double {
        Double(duration.components.seconds) + Double(duration.components.attoseconds) / 1e18
    }

    private static func seconds(_ duration: Duration) -> String {
        String(format: "%.2f s", secondsValue(duration))
    }

    private static func milliseconds(_ duration: Duration) -> String {
        String(format: "%.3f ms", secondsValue(duration) * 1000)
    }

    private static func mib(_ bytes: Int) -> String { "\(bytes >> 20) MiB" }

    private static func gib(_ bytes: Int) -> String { String(format: "%.2f GiB", Double(bytes) / Double(1 << 30)) }
}

/// The process's physical footprint, the number Activity Monitor shows as Memory.
func physFootprint() -> Int {
    var info = task_vm_info_data_t()
    var count = mach_msg_type_number_t(MemoryLayout<task_vm_info_data_t>.size / MemoryLayout<integer_t>.size)
    let result = withUnsafeMutablePointer(to: &info) {
        $0.withMemoryRebound(to: integer_t.self, capacity: Int(count)) {
            task_info(mach_task_self_, task_flavor_t(TASK_VM_INFO), $0, &count)
        }
    }
    return result == KERN_SUCCESS ? Int(info.phys_footprint) : 0
}
