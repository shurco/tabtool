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

    /// Calls `body` with the bytes of `range`, clamped to the file, through a
    /// temporary mapping of just that range.
    ///
    /// Pages read through the long-lived mapping stay resident, and `madvise`
    /// does not release them on macOS. Unmapping the window does, so a scan of
    /// a multi-gigabyte file keeps a small resident size.
    public func withWindow<R>(_ range: Range<Int>,
                              _ body: (UnsafeRawBufferPointer) throws -> R) rethrows -> R {
        let lower = min(max(range.lowerBound, 0), size)
        let upper = min(max(range.upperBound, lower), size)
        let aligned = lower / Int(getpagesize()) * Int(getpagesize())
        guard upper > lower,
              let window = mmap(nil, upper - aligned, PROT_READ, MAP_PRIVATE, descriptor, off_t(aligned)),
              window != MAP_FAILED else {
            return try withBytes(range, body)
        }
        defer { munmap(window, upper - aligned) }
        madvise(window, upper - aligned, MADV_SEQUENTIAL)
        return try body(UnsafeRawBufferPointer(start: window + (lower - aligned), count: upper - lower))
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
