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

    /// The system's language list from the global defaults domain.
    ///
    /// `Locale.preferredLanguages` follows the per-app override `applyOverride`
    /// writes, so it only serves as the fallback when the global list is absent.
    nonisolated static func systemLanguages(
        globalDomain: [String: Any]? = UserDefaults.standard.persistentDomain(forName: UserDefaults.globalDomain),
        preferred: [String] = Locale.preferredLanguages
    ) -> [String] {
        globalDomain?[appleLanguagesKey] as? [String] ?? preferred
    }

    /// Whether the first language is Chinese, which makes CSV detection try GBK before CP1252.
    nonisolated static func systemPrefersChinese(in languages: [String] = systemLanguages()) -> Bool {
        languages.first?.lowercased().hasPrefix("zh") == true
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
