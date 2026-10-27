import Foundation
import Testing
@testable import TabToolKit

private func scan(_ bytes: [UInt8], mode: RowScanner.Mode,
                  configuration: CSVConfiguration = .init(), cuts: [Int] = []) -> RowIndex {
    var scanner = RowScanner(configuration: configuration)
    scanner.mode = mode
    var index = RowIndex()
    let bounds = [0] + cuts + [bytes.count]
    for (offset, pair) in zip(bounds, bounds.dropFirst()).enumerated() {
        bytes[pair.0..<pair.1].withUnsafeBytes {
            scanner.scan($0, base: pair.0, into: &index, final: offset == bounds.count - 2)
        }
    }
    return index
}

@Suite struct RowScannerSIMDTests {
    @Test(arguments: 1...200)
    func simdMatchesScalarOnFuzz(_ seed: Int) throws {
        let bytes = RowScannerFuzz.makeInput(seed: UInt64(seed), rows: 400)
        let scalar = scan(bytes, mode: .scalar)
        #expect(scan(bytes, mode: .simd) == scalar)
        #expect(scan(bytes, mode: .simd, cuts: [37, 64, 65, 200, 1001]) == scalar)
        try expectIndexMatchesParser(bytes, scalar)
    }

    @Test func simdHandlesBlockBoundaryCases() throws {
        let patterns = ["\r\n", "\r", "\n", "\"", "\"\"", "\"x\ny\"", ",", "\"\r\n\"", "\r\r\n"]
        for pattern in patterns {
            for length in 0...130 {
                for position in [0, 62, 63, 64, 65, 126, 127, 128].filter({ $0 <= length }) {
                    var text = String(repeating: "a", count: position) + pattern
                    text += String(repeating: "b", count: length - position)
                    let bytes = Array(text.utf8)
                    let scalar = scan(bytes, mode: .scalar)
                    #expect(scan(bytes, mode: .simd) == scalar, "\(pattern.debugDescription) at \(position)")
                }
            }
        }
    }

    @Test func everyLengthMatchesScalar() {
        let unit = Array("ab,\"c\r\nd\"\r\n\n".utf8)
        let bytes = (0..<20).flatMap { _ in unit }
        for length in 0...bytes.count {
            let slice = Array(bytes[..<length])
            #expect(scan(slice, mode: .simd) == scan(slice, mode: .scalar))
        }
    }

    @Test func backslashConfigUsesScalar() {
        let configuration = CSVConfiguration(escapeCharacter: "\\")
        let text = String(repeating: "x", count: 62) + "\"a\\\"b\",c\nd" + String(repeating: "y", count: 80)
        let bytes = Array(text.utf8)
        let forced = scan(bytes, mode: .simd, configuration: configuration)
        #expect(forced == scan(bytes, mode: .scalar, configuration: configuration))
        #expect(forced.rowCount == 2)
    }

    @Test func quotelessConfigMatchesScalar() {
        let configuration = CSVConfiguration(quoteCharacter: "")
        let bytes = RowScannerFuzz.makeInput(seed: 7, rows: 300)
        #expect(scan(bytes, mode: .simd, configuration: configuration)
            == scan(bytes, mode: .scalar, configuration: configuration))
    }

    @Test func simdScanThroughput() {
        let unit = Array("12345.5,text value,\"quoted, field\",abc\r\n".utf8)
        #if DEBUG
        let size = 4 << 20
        #else
        let size = 256 << 20
        #endif
        let rows = size / unit.count
        var bytes: [UInt8] = []
        bytes.reserveCapacity(rows * unit.count)
        for _ in 0..<rows { bytes.append(contentsOf: unit) }
        let configuration = CSVConfiguration()
        let seconds = bytes.withUnsafeBytes { buffer -> Double in
            var scanner = RowScanner(configuration: configuration)
            var index = RowIndex()
            let clock = ContinuousClock()
            let elapsed = clock.measure {
                scanner.scan(buffer, base: 0, into: &index, final: true)
            }
            #expect(index.rowCount == rows)
            return Double(elapsed.components.seconds) + Double(elapsed.components.attoseconds) / 1e18
        }
        let rate = Double(bytes.count) / seconds / 1e9
        print("SIMD scan throughput: \(rate) GB/s")
        #if !DEBUG
        #expect(rate >= 1.0)
        #endif
    }
}
