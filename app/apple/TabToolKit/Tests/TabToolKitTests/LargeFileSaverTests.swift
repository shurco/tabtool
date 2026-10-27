import Foundation
import Testing
@testable import TabToolKit

private func makeSource(_ bytes: [UInt8], _ dir: TempDirectory,
                        _ configuration: CSVConfiguration = CSVConfiguration()) throws
    -> LargeCSVSource {
    let file = try MappedFile(url: dir.file("source.csv", bytes))
    let indexer = LargeFileIndexer(file: file, configuration: configuration, stepBytes: 4096,
                                   progressInterval: 0)
    indexer.start { _ in }
    indexer.wait()
    return LargeCSVSource(file: file, configuration: configuration, indexer: indexer)
}

/// Saves `source` and returns the output bytes.
@discardableResult
private func saved(_ source: LargeCSVSource, _ dir: TempDirectory,
                   progress: @Sendable (Double) -> Void = { _ in }) throws -> [UInt8] {
    let output = dir.url.appendingPathComponent("out.csv")
    let count = try LargeFileSaver.write(source.saveSnapshot(), to: output, bufferBytes: 64,
                                         progress: progress)
    let data = try Data(contentsOf: output)
    #expect(count == data.count)
    return [UInt8](data)
}

private func bytes(_ text: String) -> [UInt8] { Array(text.utf8) }

@Suite struct LargeFileSaverTests {
    @Test(arguments: [
        "a,b\n1,2\n3,4\n",
        "a,b\r\n1,2\r\n3,4\r\n",
        "a,b\n1,2\n3,4",
        "a,\"x\ny\"\r1,2\r",
        "\u{FEFF}a,b\n1,2\n",
        ""
    ])
    func noEditsIsByteIdentical(_ text: String) throws {
        let dir = TempDirectory()
        let source = try makeSource(bytes(text), dir)
        #expect(try saved(source, dir) == bytes(text))
    }

    @Test func editsOnlyChangeEditedRows() throws {
        let dir = TempDirectory()
        let lines = (0..<200).map { "r\($0),plain\($0),\"q\($0)\"" }
        let source = try makeSource(bytes(lines.joined(separator: "\n") + "\n"), dir)
        try source.setCell("changed", row: 70, column: 1)
        try source.setCell("x", row: 130, column: 0)
        var expected = lines
        expected[70] = "\"r70\",\"changed\",\"q70\""
        expected[130] = "\"x\",\"plain130\",\"q130\""
        #expect(try saved(source, dir) == bytes(expected.joined(separator: "\n") + "\n"))
    }

    @Test func editedRowKeepsItsTerminator() throws {
        let dir = TempDirectory()
        let source = try makeSource(bytes("a\r\nb\nc\rd"), dir)
        for row in 0..<4 { try source.setCell("X", row: row, column: 0) }
        #expect(try saved(source, dir) == bytes("\"X\"\r\n\"X\"\n\"X\"\r\"X\""))
    }

    @Test func editedRowZeroKeepsBOM() throws {
        let dir = TempDirectory()
        let source = try makeSource(bytes("\u{FEFF}a,b\n1,2\n"), dir)
        try source.setCell("z", row: 0, column: 0)
        #expect(try saved(source, dir) == bytes("\u{FEFF}\"z\",\"b\"\n1,2\n"))
    }

    @Test func raggedShortRowGetsPadded() throws {
        let dir = TempDirectory()
        let source = try makeSource(bytes("a,b,c,d,e,f\nx,y\n"), dir)
        try source.setCell("Z", row: 1, column: 5)
        #expect(try saved(source, dir) == bytes("a,b,c,d,e,f\n\"x\",\"y\",\"\",\"\",\"\",\"Z\"\n"))
    }

    @Test func quotedMultilineNeighborIsUntouched() throws {
        let dir = TempDirectory()
        let text = "a,b\n\"x\ny, \"\"q\"\"\",2\nlast,row\n"
        let source = try makeSource(bytes(text), dir)
        try source.setCell("L", row: 2, column: 0)
        #expect(try saved(source, dir) == bytes("a,b\n\"x\ny, \"\"q\"\"\",2\n\"L\",\"row\"\n"))
    }

    @Test func editedRowWithQuotesOffIsWrittenPlain() throws {
        let dir = TempDirectory()
        let configuration = CSVConfiguration(quoteCharacter: "", escapeCharacter: "")
        let source = try makeSource(bytes("a,b\n1,2\n"), dir, configuration)
        try source.setCell("new", row: 1, column: 0)
        #expect(try saved(source, dir) == bytes("a,b\nnew,2\n"))
    }

    @Test func progressReachesOne() throws {
        let dir = TempDirectory()
        let source = try makeSource(bytes((0..<500).map { "row\($0),v" }.joined(separator: "\n")), dir)
        try source.setCell("e", row: 300, column: 0)
        let seen = Locked<[Double]>([])
        try saved(source, dir) { fraction in seen.withValue { $0.append(fraction) } }
        let values = seen.withValue { $0 }
        #expect(values.last == 1)
        #expect(values == values.sorted())
    }

    /// A random fully quoted source is exactly what `CSVWriter` produces, so the
    /// saver's output must equal the writer's output for the edited table.
    @Test(arguments: 1...60)
    func matchesInMemoryWriterOnRandomEdits(seed: Int) throws {
        var generator = SeededGenerator(seed: UInt64(seed))
        let alphabet = ["a", "b", "", " ", ",", "\"", "\n", "\r\n", "é", "日本", "x y", "\\"]
        func randomText() -> String {
            (0..<Int.random(in: 0...3, using: &generator))
                .map { _ in alphabet.randomElement(using: &generator)! }.joined()
        }
        let columns = Int.random(in: 1...6, using: &generator)
        let rowCount = Int.random(in: 1...300, using: &generator)
        var rows = (0..<rowCount).map { _ in (0..<columns).map { _ in randomText() } }
        let configuration = CSVConfiguration()
        let original = CSVTable(rows: rows, headers: nil)
        let dir = TempDirectory()
        let source = try makeSource(
            Array(CSVWriter(table: original, header: nil, configuration: configuration).data()),
            dir, configuration)
        #expect(source.rowCount == rowCount)

        for _ in 0..<Int.random(in: 0...40, using: &generator) {
            let row = Int.random(in: 0..<rowCount, using: &generator)
            let column = Int.random(in: 0..<columns, using: &generator)
            let text = randomText()
            try source.setCell(text, row: row, column: column)
            rows[row][column] = text
        }
        let expected = try CSVWriter(table: CSVTable(rows: rows, headers: nil), header: nil,
                                     configuration: configuration).data()
        #expect(try saved(source, dir) == [UInt8](expected))
    }

    @Test func cancellationStopsAndRemovesPartialOutput() async throws {
        let dir = TempDirectory()
        let source = try makeSource(bytes((0..<2000).map { "row\($0),v" }.joined(separator: "\n")), dir)
        let snapshot = source.saveSnapshot()
        let output = dir.url.appendingPathComponent("out.csv")
        let task = Task.detached {
            try LargeFileSaver.write(snapshot, to: output, bufferBytes: 16) { _ in
                withUnsafeCurrentTask { $0?.cancel() }
            }
        }
        await #expect(throws: CancellationError.self) { try await task.value }
        #expect(!FileManager.default.fileExists(atPath: output.path))
    }

    @Test func saveReplacesTheOriginal() throws {
        let dir = TempDirectory()
        let source = try makeSource(bytes("a,b\n1,2\n"), dir)
        try source.setCell("9", row: 1, column: 1)
        let target = dir.url.appendingPathComponent("source.csv")
        try LargeFileSaver.save(source.saveSnapshot(), replacing: target,
                                availableCapacity: { _ in 1 << 40 }, progress: { _ in })
        #expect(try Data(contentsOf: target) == Data(bytes("a,b\n\"1\",\"9\"\n")))
    }

    @Test func savingToANewFileGivesItTheDefaultPermissions() throws {
        let dir = TempDirectory()
        let source = try makeSource(bytes("a,b\n"), dir)
        let target = dir.url.appendingPathComponent("copy.csv")
        try LargeFileSaver.save(source.saveSnapshot(), replacing: target,
                                availableCapacity: { _ in 1 << 40 }, progress: { _ in })
        let plain = dir.url.appendingPathComponent("plain.csv")
        FileManager.default.createFile(atPath: plain.path, contents: Data())
        func permissions(_ url: URL) throws -> Int? {
            try FileManager.default.attributesOfItem(atPath: url.path)[.posixPermissions] as? Int
        }
        #expect(try permissions(target) == permissions(plain))
    }

    @Test func saveFailsWithoutTwiceTheFreeSpace() throws {
        let dir = TempDirectory()
        let text = bytes("a,b\n1,2\n")
        let source = try makeSource(text, dir)
        let target = dir.url.appendingPathComponent("source.csv")
        let snapshot = source.saveSnapshot()
        #expect(throws: LargeFileSaveError.insufficientDiskSpace) {
            try LargeFileSaver.save(snapshot, replacing: target,
                                    availableCapacity: { _ in Int64(text.count * 2 - 1) },
                                    progress: { _ in })
        }
        #expect(try Data(contentsOf: target) == Data(text))
        try LargeFileSaver.save(snapshot, replacing: target,
                                availableCapacity: { _ in Int64(text.count * 2) }, progress: { _ in })
    }

    @Test func saveFailsWhileIndexing() throws {
        let dir = TempDirectory()
        let file = try MappedFile(url: dir.file("source.csv", bytes("a\nb\n")))
        let indexer = LargeFileIndexer(file: file, configuration: CSVConfiguration())
        let source = LargeCSVSource(file: file, configuration: CSVConfiguration(), indexer: indexer)
        #expect(throws: LargeFileSaveError.indexingNotFinished) {
            try LargeFileSaver.save(source.saveSnapshot(), replacing: dir.url.appendingPathComponent("source.csv"),
                                    availableCapacity: { _ in 1 << 40 }, progress: { _ in })
        }
    }

    @Test func saveFailsWhenTheFileChanged() throws {
        let dir = TempDirectory()
        let source = try makeSource(bytes("a\nb\n"), dir)
        let target = dir.url.appendingPathComponent("source.csv")
        try Data(bytes("short")).write(to: target)
        #expect(throws: LargeFileSaveError.sourceChanged) {
            try LargeFileSaver.save(source.saveSnapshot(), replacing: target,
                                    availableCapacity: { _ in 1 << 40 }, progress: { _ in })
        }
    }
}
