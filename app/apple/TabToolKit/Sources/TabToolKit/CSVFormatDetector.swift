import Foundation

/// Picks the most likely `CSVConfiguration` for raw data.
///
/// Detects the format of a CSV file by scoring candidate configurations:
///
/// - Eleven candidate configurations are scored in a fixed order.
/// - Each candidate parses at most five rows.
/// - Every parsed row adds one point.
/// - Rows that are not longer than the first row add their cell count.
/// - A candidate whose rows all have the same cell count adds five points and
///   marks the first row as a header.
/// - Numeric cells add one point each, up to five; a numeric or empty first-row
///   cell clears the header flag.
/// - A parse error skips the candidate; a decoding error ends the encoding
///   attempt. Scores are never reset between attempts.
///
/// Ties keep the earliest candidate, so the result is deterministic.
public struct CSVFormatDetector: Sendable {
    private static let sampleRowLimit = 5
    private static let numberBonusLimit = 5
    private static let equalLengthBonus = 5

    private let data: Data
    private let prefersChineseEncoding: Bool

    public init(data: Data, prefersChineseEncoding: Bool = false) {
        self.data = data
        self.prefersChineseEncoding = prefersChineseEncoding
    }

    /// Returns the best configuration found by the heuristic.
    ///
    /// Detection never throws: a configuration is returned even when the data
    /// cannot be decoded or parsed with it, and the reader reports that error
    /// later.
    public func detect() -> CSVConfiguration {
        let configurations = Self.configurations
        let encodings = Self.encodings(preferringChinese: prefersChineseEncoding)
        var scores = Array(repeating: 0, count: configurations.count)
        var headerFlags = Array(repeating: false, count: configurations.count)
        var detectedEncoding = encodings.last ?? .utf8

        for encoding in encodings {
            var decoded = true
            for (index, template) in configurations.enumerated() {
                var configuration = template
                configuration.encoding = encoding
                guard var parser = try? CSVParser(data: data, configuration: configuration) else {
                    // Every candidate decodes the same bytes, so one decoding
                    // error ends the attempt and the next encoding is tried.
                    decoded = false
                    break
                }
                guard let rows = Self.readRows(from: &parser) else {
                    continue
                }
                scores[index] += rows.count
                Self.scoreRowLengths(rows, score: &scores[index], headerFlag: &headerFlags[index])
                Self.scoreNumbers(rows, decimalMark: configuration.decimalMark,
                                  score: &scores[index], headerFlag: &headerFlags[index])
            }
            if decoded {
                detectedEncoding = encoding
                break
            }
        }

        // A strict comparison keeps the earlier candidate on a tie.
        var bestIndex = 0
        for index in scores.indices.dropFirst() where scores[index] > scores[bestIndex] {
            bestIndex = index
        }

        var best = configurations[bestIndex]
        best.encoding = detectedEncoding
        best.firstRowAsHeader = headerFlags[bestIndex]
        return best
    }

    /// The candidates in scoring order, which differs
    /// from the `config1`...`config11` numbering: 1, 10, 6, 2, 5, 7, 3, 8, 4,
    /// 9, 11.
    private static let configurations: [CSVConfiguration] = [
        CSVConfiguration(columnSeparator: ",", decimalMark: "."),
        CSVConfiguration(columnSeparator: ",", quoteCharacter: "\"",
                         escapeCharacter: "\\", decimalMark: "."),
        CSVConfiguration(columnSeparator: ",", quoteCharacter: "", decimalMark: "."),
        CSVConfiguration(columnSeparator: ";", decimalMark: ","),
        CSVConfiguration(columnSeparator: ";", quoteCharacter: "\"",
                         escapeCharacter: "\\", decimalMark: ","),
        CSVConfiguration(columnSeparator: ";", quoteCharacter: "", decimalMark: ","),
        CSVConfiguration(columnSeparator: "\t", decimalMark: "."),
        CSVConfiguration(columnSeparator: "\t", quoteCharacter: "", decimalMark: "."),
        CSVConfiguration(columnSeparator: "\t", decimalMark: ","),
        CSVConfiguration(columnSeparator: "\t", quoteCharacter: "", decimalMark: ","),
        CSVConfiguration(columnSeparator: ",", decimalMark: ",")
    ]

    private static func encodings(preferringChinese: Bool) -> [String.Encoding] {
        if preferringChinese {
            return [.utf8, SupportedEncodings.gbk, .windowsCP1252, .macOSRoman]
        }
        return [.utf8, .windowsCP1252, .macOSRoman, SupportedEncodings.gbk]
    }

    /// Reads at most `sampleRowLimit` rows, or returns `nil` on a parse error.
    private static func readRows(from parser: inout CSVParser) -> [[String]]? {
        var rows: [[String]] = []
        while rows.count < sampleRowLimit {
            do {
                guard let row = try parser.nextRow() else { break }
                rows.append(row)
            } catch {
                return nil
            }
        }
        return rows
    }

    private static func scoreRowLengths(_ rows: [[String]], score: inout Int,
                                        headerFlag: inout Bool) {
        guard let firstRow = rows.first else { return }
        let firstRowCellCount = firstRow.count
        var allRowsHaveSameLength = true
        for row in rows.dropFirst() {
            allRowsHaveSameLength = allRowsHaveSameLength && row.count == firstRowCellCount
            if row.count <= firstRowCellCount {
                score += row.count
            }
        }
        if allRowsHaveSameLength {
            score += equalLengthBonus
            headerFlag = true
        }
    }

    private static func scoreNumbers(_ rows: [[String]], decimalMark: String,
                                     score: inout Int, headerFlag: inout Bool) {
        var numberCount = 0
        for (rowIndex, row) in rows.enumerated() {
            for cell in row {
                if CSVNumber.isNumeric(cell, decimalMark: decimalMark) {
                    numberCount += 1
                    if rowIndex == 0 {
                        headerFlag = false
                    }
                } else if rowIndex == 0 && cell.isEmpty {
                    headerFlag = false
                }
            }
        }
        score += min(numberCount, numberBonusLimit)
    }
}
