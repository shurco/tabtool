import Foundation

/// Limits that define large file mode.
public enum LargeFile {
    /// Files at or above this size open through the memory-mapped path.
    public static let thresholdBytes: Int64 = 100 * 1_048_576

    /// Rows per index checkpoint, and per decoded block.
    public static let blockRows = 64

    /// Bytes of the file head used to detect the format.
    public static let sampleBytes = 1_048_576

    /// Columns beyond this count are hidden; their bytes are never touched.
    public static let maxColumns = 2_048

    /// The largest number of cells a single copy may produce.
    public static let maxCopyCells = 1_000_000
}
