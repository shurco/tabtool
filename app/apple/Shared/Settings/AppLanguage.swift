import Foundation

/// The interface languages the app ships, plus following the system.
enum AppLanguage: String, CaseIterable, Identifiable {
    case system, en, fr, es, de, it, pl, be, uk
    case zhHans = "zh-Hans"
    case ja, ko

    private static let appleLanguagesKey = "AppleLanguages"

    var id: String { rawValue }

    /// The language's own name, shown as is in every interface language; `nil` for `.system`.
    var nativeName: String? {
        switch self {
        case .system: nil
        case .en: "English"
        case .fr: "Français"
        case .es: "Español"
        case .de: "Deutsch"
        case .it: "Italiano"
        case .pl: "Polski"
        case .be: "Беларуская"
        case .uk: "Українська"
        case .zhHans: "简体中文"
        case .ja: "日本語"
        case .ko: "한국어"
        }
    }

    /// Writes the per-app language override read at the next launch.
    func applyOverride(to defaults: UserDefaults = .standard) {
        if self == .system {
            defaults.removeObject(forKey: Self.appleLanguagesKey)
        } else {
            defaults.set([rawValue], forKey: Self.appleLanguagesKey)
        }
    }
}
