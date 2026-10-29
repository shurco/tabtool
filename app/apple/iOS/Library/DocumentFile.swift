import Foundation

/// A document on disk, as listed by the library.
struct DocumentFile: Identifiable, Hashable {
    let url: URL
    let modified: Date
    let size: Int64

    var id: URL { url }
    var name: String { url.lastPathComponent }
}

/// Filename rules for new and imported documents.
///
/// Names are compared case-insensitively because the app's container lives on
/// a case-insensitive volume.
enum DocumentNaming {
    static let newDocumentBase = "Untitled"

    /// "Untitled.csv", then "Untitled 2.csv", and so on.
    static func newDocumentName(existing: Set<String>, fileExtension: String = "csv") -> String {
        let taken = Set(existing.map { $0.lowercased() })
        var candidate = "\(newDocumentBase).\(fileExtension)"
        var index = 2
        while taken.contains(candidate.lowercased()) {
            candidate = "\(newDocumentBase) \(index).\(fileExtension)"
            index += 1
        }
        return candidate
    }

    /// The proposed name, or the same name with " 2", " 3" before the extension.
    static func availableName(for proposed: String, existing: Set<String>) -> String {
        let url = URL(fileURLWithPath: proposed)
        let pathExtension = url.pathExtension
        let base = url.deletingPathExtension().lastPathComponent
        let taken = Set(existing.map { $0.lowercased() })
        var candidate = proposed
        var index = 2
        while taken.contains(candidate.lowercased()) {
            candidate = pathExtension.isEmpty ? "\(base) \(index)" : "\(base) \(index).\(pathExtension)"
            index += 1
        }
        return candidate
    }
}
