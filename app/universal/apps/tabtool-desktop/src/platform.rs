//! Platform services and process-wide values the shell needs.

use std::sync::Arc;

use tabtool_core::Locale;
use tabtool_document::{StdStorage, Storage};
use tabtool_i18n::{Language, language_from_system};
use tabtool_platform::{DesktopPlatform, Settings, SystemInfo};

/// The storage every document window shares.
///
/// The trait object is `Send + Sync` because deferred viewport callbacks are
/// stored by egui and may run on another thread.
pub type SharedStorage = Arc<dyn Storage + Send + Sync>;

/// The filesystem storage used by the real app.
pub fn standard_storage() -> SharedStorage {
    Arc::new(StdStorage)
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
    resolve_language(settings.app_language, &platform.languages())
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
