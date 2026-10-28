import Foundation
import Testing
@testable import TabTool

@Suite struct AppLanguageTests {
    @Test func allCasesAreSystemPlusElevenUniqueLanguages() {
        #expect(AppLanguage.allCases.count == 12)
        #expect(Set(AppLanguage.allCases.map(\.rawValue)).count == 12)
        #expect(AppLanguage.allCases.first == .system)
    }

    @Test func onlySystemHasNoNativeName() {
        for language in AppLanguage.allCases {
            #expect((language.nativeName == nil) == (language == .system))
        }
        #expect(AppLanguage.zhHans.nativeName == "简体中文")
    }

    @Test(arguments: [(AppLanguage.fr, "fr"), (.zhHans, "zh-Hans")])
    func applyingALanguageWritesAppleLanguages(language: AppLanguage, code: String) {
        let store = TestDefaults()
        language.applyOverride(to: store.defaults)
        #expect(store.persistentValues["AppleLanguages"] as? [String] == [code])
    }

    @Test func applyingSystemRemovesTheOverride() {
        let store = TestDefaults()
        AppLanguage.de.applyOverride(to: store.defaults)
        AppLanguage.system.applyOverride(to: store.defaults)
        #expect(store.persistentValues["AppleLanguages"] == nil)
    }

    @Test func bundleLocalizationsMatchTheSupportedLanguages() {
        let declared = Set(Bundle.main.infoDictionary?["CFBundleLocalizations"] as? [String] ?? [])
        #expect(declared == Set(AppLanguage.allCases.filter { $0 != .system }.map(\.rawValue)))
    }

    @Test(arguments: [["zh-Hans-CN"], ["zh-Hant"], ["zh"], ["ZH-hans", "en"]])
    func aChineseFirstLanguagePrefersTheChineseEncoding(languages: [String]) {
        #expect(AppLanguage.systemPrefersChinese(in: languages))
    }

    @Test(arguments: [["en"], ["ru-RU"], ["en", "zh-Hans"], []])
    func otherFirstLanguagesDoNotPreferTheChineseEncoding(languages: [String]) {
        #expect(!AppLanguage.systemPrefersChinese(in: languages))
    }

    @Test func systemLanguagesComeFromTheGlobalDomainNotTheAppOverride() {
        let global: [String: Any] = ["AppleLanguages": ["zh-Hans-CN", "en"]]
        #expect(AppLanguage.systemLanguages(globalDomain: global, preferred: ["en"]) == ["zh-Hans-CN", "en"])
    }

    @Test func systemLanguagesFallBackToThePreferredList() {
        #expect(AppLanguage.systemLanguages(globalDomain: nil, preferred: ["fr"]) == ["fr"])
        #expect(AppLanguage.systemLanguages(globalDomain: [:], preferred: ["fr"]) == ["fr"])
    }
}
