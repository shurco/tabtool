import Foundation

/// A progress report from `LargeFileIndexer`.
public struct IndexProgress: Sendable {
    public let fraction: Double
    public let rowCount: Int
    public let maxFieldCount: Int
    public let isFinished: Bool
}

/// Scans a mapped file on a background queue and publishes the growing `RowIndex`.
///
/// The file is read in steps of `stepBytes` into one reused buffer. After
/// every step the index so far is available through `snapshot`, so a reader
/// can show the first rows while the scan continues.
public final class LargeFileIndexer: @unchecked Sendable {
    private let file: MappedFile
    private let configuration: CSVConfiguration
    private let stepBytes: Int
    private let progressInterval: TimeInterval
    private let finished = DispatchGroup()

    // The lock guards the published index and the two flags.
    private let lock = NSLock()
    private var published = RowIndex()
    private var finishedScan = false
    private var cancelled = false
    private var started = false

    public init(file: MappedFile, configuration: CSVConfiguration,
                stepBytes: Int = 8 << 20, progressInterval: TimeInterval = 0.1) {
        self.file = file
        self.configuration = configuration
        self.stepBytes = max(stepBytes, 1)
        self.progressInterval = progressInterval
    }

    /// The index of the bytes scanned so far.
    public var snapshot: RowIndex { locked { published } }

    /// Whether the whole file has been scanned.
    public var isFinished: Bool { locked { finishedScan } }

    /// Starts scanning once. `onProgress` runs on the scanning queue: right
    /// after the first step, then at most every `progressInterval`, and at the end.
    public func start(onProgress: @escaping @Sendable (IndexProgress) -> Void) {
        let alreadyStarted = locked { () -> Bool in
            defer { started = true }
            return started
        }
        guard !alreadyStarted else { return }
        finished.enter()
        DispatchQueue.global(qos: .userInitiated).async { [self] in
            defer { finished.leave() }
            run(onProgress)
        }
    }

    /// Asks the scan to stop after the current step.
    public func cancel() {
        locked { cancelled = true }
    }

    /// Blocks until the scan has finished or stopped.
    public func wait() {
        finished.wait()
    }

    private func run(_ onProgress: @Sendable (IndexProgress) -> Void) {
        var scanner = RowScanner(configuration: configuration)
        // `index` holds only the checkpoints of the current step; they move
        // into `published`. Sharing the whole growing array with `published`
        // made every step copy it, and the freed copies stayed with malloc.
        var index = RowIndex()
        var buffer: [UInt8] = []
        var offset = 0
        var lastReport: Date?
        repeat {
            if locked({ cancelled }) { return }
            let end = min(offset + stepBytes, file.size)
            let isLast = end == file.size
            file.read(offset..<end, into: &buffer) {
                scanner.scan($0, base: offset, into: &index, final: isLast)
            }
            offset = end
            locked {
                published.extend(with: index)
                finishedScan = isLast
            }
            index.checkpoints.removeAll(keepingCapacity: true)
            let now = Date()
            if isLast || lastReport.map({ now.timeIntervalSince($0) >= progressInterval }) ?? true {
                lastReport = now
                onProgress(IndexProgress(fraction: file.size == 0 ? 1 : Double(offset) / Double(file.size),
                                         rowCount: index.rowCount, maxFieldCount: index.maxFieldCount,
                                         isFinished: isLast))
            }
        } while offset < file.size
    }

    private func locked<R>(_ body: () -> R) -> R {
        lock.lock()
        defer { lock.unlock() }
        return body()
    }
}
