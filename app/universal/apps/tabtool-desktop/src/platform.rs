//! Platform services and process-wide values the shell needs.

use std::sync::Arc;

use tabtool_core::Locale;
use tabtool_document::{StdStorage, Storage};
use tabtool_platform::{DesktopPlatform, FileDialogs, SystemInfo};

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

/// The system locale, or `en` when the platform reports one the ICU parser
/// does not accept.
pub fn system_locale(platform: &DesktopPlatform) -> Locale {
    Locale::try_from_str(&platform.locale())
        .unwrap_or_else(|_| Locale::try_from_str("en").expect("English is a valid ICU locale"))
}

#[cfg(test)]
mod tests {
    use tabtool_i18n::Language;
    use tabtool_settings::Settings;

    use tabtool_platform::prefers_chinese_encoding;

    #[test]
    fn an_override_language_wins_but_the_system_still_picks_the_detector() {
        let settings = Settings {
            app_language: Language::Fr,
            ..Settings::default()
        };
        let system = ["zh-Hans".to_owned()];

        assert_eq!(settings.interface_language(&system), Language::Fr);
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
