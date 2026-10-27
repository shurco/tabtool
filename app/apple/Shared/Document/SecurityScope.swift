import Foundation

/// Holds security-scoped access to a file for as long as the object lives.
final class SecurityScope {
    private let url: URL
    private let isActive: Bool

    init(_ url: URL) {
        self.url = url
        isActive = url.startAccessingSecurityScopedResource()
    }

    deinit {
        if isActive { url.stopAccessingSecurityScopedResource() }
    }
}
