import Foundation

/// Size, modification time and inode of a file, used to notice outside changes.
public struct FileIdentity: Equatable, Sendable {
    public let size: Int
    public let modifiedNanoseconds: Int64
    public let inode: UInt64

    init(_ info: stat) {
        size = Int(info.st_size)
        modifiedNanoseconds = Int64(info.st_mtimespec.tv_sec) * 1_000_000_000
            + Int64(info.st_mtimespec.tv_nsec)
        inode = UInt64(info.st_ino)
    }
}

/// Whether the file on disk still matches the mapped one.
public enum Staleness: Equatable, Sendable {
    case unchanged
    case appended
    case replacedOrTruncated
}

/// A read-only memory mapping of a file.
///
/// The descriptor, base pointer and identity never change after `init`, so the
/// type is safe to share between threads. Reads past the mapped size are
/// clamped; a file truncated by another process can still fault on access, so
/// callers check `isReadable` first.
public final class MappedFile: @unchecked Sendable {
    public let size: Int
    public let identity: FileIdentity
    private let url: URL
    private let descriptor: Int32
    private let base: UnsafeMutableRawPointer?

    public init(url: URL) throws {
        let descriptor = open(url.path, O_RDONLY)
        guard descriptor >= 0 else { throw CocoaError(.fileReadNoSuchFile) }
        var info = stat()
        guard fstat(descriptor, &info) == 0 else {
            close(descriptor)
            throw CocoaError(.fileReadUnknown)
        }
        let identity = FileIdentity(info)
        var base: UnsafeMutableRawPointer?
        if identity.size > 0 {
            let mapped = mmap(nil, identity.size, PROT_READ, MAP_PRIVATE, descriptor, 0)
            guard mapped != MAP_FAILED else {
                close(descriptor)
                throw CocoaError(.fileReadUnknown)
            }
            base = mapped
        }
        self.url = url
        self.descriptor = descriptor
        self.base = base
        self.size = identity.size
        self.identity = identity
    }

    deinit {
        if let base { munmap(base, size) }
        close(descriptor)
    }

    /// Calls `body` with the bytes of `range`, clamped to the file.
    public func withBytes<R>(_ range: Range<Int>,
                             _ body: (UnsafeRawBufferPointer) throws -> R) rethrows -> R {
        let lower = min(max(range.lowerBound, 0), size)
        let upper = min(max(range.upperBound, lower), size)
        guard let base, upper > lower else {
            return try body(UnsafeRawBufferPointer(start: nil, count: 0))
        }
        return try body(UnsafeRawBufferPointer(start: base + lower, count: upper - lower))
    }

    /// Reads the bytes of `range`, clamped to the file, into `buffer` and
    /// calls `body` with the bytes read; fewer than asked when the file was
    /// truncated meanwhile. `buffer` grows as needed and can be reused.
    ///
    /// For sequential scans: walking a mapping faults pages in one by one
    /// (about 0.7 GB/s measured) and keeps them resident, while `pread` runs
    /// at 4-9 GB/s, holds only `buffer` and never faults on a truncated file.
    public func read<R>(_ range: Range<Int>, into buffer: inout [UInt8],
                        _ body: (UnsafeRawBufferPointer) throws -> R) rethrows -> R {
        let lower = min(max(range.lowerBound, 0), size)
        let wanted = min(max(range.upperBound, lower), size) - lower
        if buffer.count < wanted { buffer = [UInt8](repeating: 0, count: wanted) }
        return try buffer.withUnsafeMutableBytes { storage in
            var done = 0
            while done < wanted {
                let count = pread(descriptor, storage.baseAddress! + done, wanted - done, off_t(lower + done))
                if count < 0 && errno == EINTR { continue }
                guard count > 0 else { break }
                done += count
            }
            return try body(UnsafeRawBufferPointer(rebasing: storage[..<done]))
        }
    }

    /// Whether the mapped bytes can still be read: the open file was neither
    /// truncated nor rewritten (it may have grown). Another file now at the
    /// path does not matter; the descriptor keeps the mapped one alive.
    public var isReadable: Bool { openFileStaleness() != .replacedOrTruncated }

    /// Compares the path and the open descriptor with the identity recorded at open.
    public func isStale() -> Staleness {
        var onPath = stat()
        guard stat(url.path, &onPath) == 0, FileIdentity(onPath).inode == identity.inode else {
            return .replacedOrTruncated
        }
        return openFileStaleness()
    }

    private func openFileStaleness() -> Staleness {
        var onDescriptor = stat()
        guard fstat(descriptor, &onDescriptor) == 0 else { return .replacedOrTruncated }
        let openIdentity = FileIdentity(onDescriptor)
        if openIdentity == identity { return .unchanged }
        return openIdentity.size > identity.size ? .appended : .replacedOrTruncated
    }
}
