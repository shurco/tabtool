import Foundation

extension Bundle {
    /// Looks up a runtime string in the String Catalog; a key without an entry reads as itself.
    func localized(_ key: String) -> String {
        localizedString(forKey: key, value: nil, table: nil)
    }
}
