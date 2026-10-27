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
/// callers check `isStale()` first.
public final class MappedFile: @unchecked Sendable {
    public enum Advice: Sendable {
        case sequential, random, dontNeed
    }

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

    /// Hints the kernel about the access pattern for `range`.
    public func advise(_ advice: Advice, range: Range<Int>) {
        let page = Int(getpagesize())
        let lower = min(max(range.lowerBound, 0), size) / page * page
        let upper = min(max(range.upperBound, 0), size)
        guard let base, upper > lower else { return }
        let flag: Int32
        switch advice {
        case .sequential: flag = MADV_SEQUENTIAL
        case .random: flag = MADV_RANDOM
        case .dontNeed: flag = MADV_DONTNEED
        }
        madvise(base + lower, upper - lower, flag)
    }

    /// Compares the open descriptor and the path with the identity recorded at open.
    public func isStale() -> Staleness {
        var onPath = stat()
        guard stat(url.path, &onPath) == 0 else { return .replacedOrTruncated }
        var onDescriptor = stat()
        guard fstat(descriptor, &onDescriptor) == 0 else { return .replacedOrTruncated }
        let pathIdentity = FileIdentity(onPath)
        let openIdentity = FileIdentity(onDescriptor)
        guard pathIdentity.inode == identity.inode,
              openIdentity.size >= identity.size else { return .replacedOrTruncated }
        if pathIdentity == identity && openIdentity == identity { return .unchanged }
        if pathIdentity.size > identity.size && openIdentity.size > identity.size { return .appended }
        return .replacedOrTruncated
    }
}
