//! Platform services and process-wide values the shell needs.

use std::sync::Arc;

use tabtool_core::Locale;
use tabtool_document::{StdStorage, Storage};
use tabtool_i18n::{Language, language_from_system};
use tabtool_platform::{DesktopPlatform, FileDialogs, Settings, SystemInfo};

/// The storage every document window shares.
///
/// The trait object is `Send + Sync` because deferred viewport callbacks are
/// stored by egui and may run on another thread.
pub type SharedStorage = Arc<dyn Storage + Send + Sync>;

/// The native file dialogs every document window shares.
///
/// The real app injects [`DesktopPlatform`]; tests inject a scripted dialog
/// source so Save As and close paths run without a user.
pub type SharedDialogs = Arc<dyn FileDialogs + Send + Sync>;

/// The filesystem storage used by the real app.
pub fn standard_storage() -> SharedStorage {
    Arc::new(StdStorage)
}

/// The real platform file dialogs.
pub fn standard_dialogs() -> SharedDialogs {
    Arc::new(DesktopPlatform)
}

/// The language the interface uses for a preference and the system languages.
///
/// [`Language::System`] follows the first system language through the catalog
/// mapping; an explicit preference always wins.
pub fn resolve_language(preference: Language, system_tags: &[String]) -> Language {
    match preference {
        Language::System => language_from_system(system_tags),
        language => language,
    }
}

/// The interface language for the current settings and platform.
pub fn interface_language(settings: &Settings, platform: &DesktopPlatform) -> Language {
    interface_language_for(settings, &platform.languages())
}

/// The interface language for a preference and already-resolved system tags.
pub fn interface_language_for(settings: &Settings, system_tags: &[String]) -> Language {
    resolve_language(settings.app_language, system_tags)
}

/// Whether the system language makes the detector try GBK before CP1252.
///
/// This follows the first system language, not the interface override: a
/// Chinese system with an English UI still prefers GBK.
pub fn prefers_chinese_encoding(system_tags: &[String]) -> bool {
    language_from_system(system_tags) == Language::ZhHans
}

/// The system locale, or `en` when the platform reports one the ICU parser
/// does not accept.
pub fn system_locale(platform: &DesktopPlatform) -> Locale {
    Locale::try_from_str(&platform.locale())
        .unwrap_or_else(|_| Locale::try_from_str("en").expect("English is a valid ICU locale"))
}

/// The system decimal separator, `,` or `.`.
pub fn decimal_separator(platform: &DesktopPlatform) -> String {
    platform.decimal_separator()
}

#[cfg(test)]
mod tests {
    use tabtool_i18n::Language;
    use tabtool_platform::Settings;

    use super::{interface_language_for, prefers_chinese_encoding};

    #[test]
    fn an_override_language_wins_but_the_system_still_picks_the_detector() {
        let settings = Settings {
            app_language: Language::Fr,
            ..Settings::default()
        };
        let system = ["zh-Hans".to_owned()];

        assert_eq!(interface_language_for(&settings, &system), Language::Fr);
        assert!(
            prefers_chinese_encoding(&system),
            "a French UI on a Chinese system must still try GBK"
        );
    }

    #[test]
    fn a_western_system_does_not_prefer_gbk() {
        let system = ["fr-FR".to_owned()];
        assert!(!prefers_chinese_encoding(&system));
    }
}
