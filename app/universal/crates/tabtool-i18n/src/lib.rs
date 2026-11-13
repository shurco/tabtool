//! The TabTool interface catalog.
//!
//! The catalog is generated from `app/apple/Shared/Localizable.xcstrings` and
//! the `strings/universal.json` overlay by `cargo run -p xtask -- i18n`.
//! English source keys double as lookup keys. Translations that are missing
//! fall back to English.

#![forbid(unsafe_code)]

mod catalog;

include!("../strings/locales.rs");

/// An interface language, or following the system.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Language {
    /// Use the system language.
    System,
    /// English.
    En,
    /// French.
    Fr,
    /// Spanish.
    Es,
    /// German.
    De,
    /// Italian.
    It,
    /// Polish.
    Pl,
    /// Belarusian.
    Be,
    /// Ukrainian.
    Uk,
    /// Simplified Chinese.
    ZhHans,
    /// Japanese.
    Ja,
    /// Korean.
    Ko,
}

impl Language {
    /// Every language, system first.
    pub const ALL: [Self; 12] = [
        Self::System,
        Self::En,
        Self::Fr,
        Self::Es,
        Self::De,
        Self::It,
        Self::Pl,
        Self::Be,
        Self::Uk,
        Self::ZhHans,
        Self::Ja,
        Self::Ko,
    ];

    /// The catalog code, or `None` for [`Language::System`].
    pub fn code(self) -> Option<&'static str> {
        match self {
            Self::System => None,
            Self::En => Some("en"),
            Self::Fr => Some("fr"),
            Self::Es => Some("es"),
            Self::De => Some("de"),
            Self::It => Some("it"),
            Self::Pl => Some("pl"),
            Self::Be => Some("be"),
            Self::Uk => Some("uk"),
            Self::ZhHans => Some("zh-Hans"),
            Self::Ja => Some("ja"),
            Self::Ko => Some("ko"),
        }
    }

    /// The language's own name, untranslated; `None` for [`Language::System`].
    pub fn native_name(self) -> Option<&'static str> {
        match self {
            Self::System => None,
            Self::En => Some("English"),
            Self::Fr => Some("Français"),
            Self::Es => Some("Español"),
            Self::De => Some("Deutsch"),
            Self::It => Some("Italiano"),
            Self::Pl => Some("Polski"),
            Self::Be => Some("Беларуская"),
            Self::Uk => Some("Українська"),
            Self::ZhHans => Some("简体中文"),
            Self::Ja => Some("日本語"),
            Self::Ko => Some("한국어"),
        }
    }

    /// The language with the given catalog code.
    pub fn from_code(code: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|language| language.code() == Some(code))
    }

    /// The value persisted in the settings file.
    pub fn persisted(self) -> &'static str {
        self.code().unwrap_or("system")
    }

    /// The language stored as `value`, if it is a known one.
    pub fn from_persisted(value: &str) -> Option<Self> {
        if value == "system" {
            Some(Self::System)
        } else {
            Self::from_code(value)
        }
    }
}

/// The translation of `key` for `language`, falling back to English.
///
/// A locale entry that is missing falls back to the English translation; a
/// key that is not in the catalog reads as itself, so a typo is visible
/// instead of an empty label. The generated catalog is checked against the
/// source by `xtask i18n --check`.
pub fn tr(language: Language, key: &str) -> &str {
    resolve(language, key, tr_in)
}

/// The translation of `key` for `language`, without falling back to English.
pub fn tr_in(language: Language, key: &str) -> Option<&'static str> {
    let code = language.code().unwrap_or("en");
    catalog::translation(key, code)
}

fn resolve(
    language: Language,
    key: &str,
    lookup: impl Fn(Language, &str) -> Option<&'static str>,
) -> &str {
    lookup(language, key)
        .or_else(|| lookup(Language::En, key))
        .unwrap_or(key)
}

/// The language with every catalog key.
pub fn keys() -> impl Iterator<Item = &'static str> {
    catalog::ENTRIES.iter().map(|entry| entry.0)
}

/// How many keys the catalog has.
pub fn key_count() -> usize {
    catalog::ENTRIES.len()
}

/// The interface language for an ordered system language list.
///
/// The first tag wins. Chinese variants all use the Simplified catalog, and an
/// unknown language falls back to English, as the app does.
pub fn language_from_system(tags: &[String]) -> Language {
    let Some(tag) = tags.first() else {
        return Language::En;
    };
    let normalized = tag.trim().to_ascii_lowercase();
    let primary = normalized.split(['-', '_']).next().unwrap_or("");
    match primary {
        "fr" => Language::Fr,
        "es" => Language::Es,
        "de" => Language::De,
        "it" => Language::It,
        "pl" => Language::Pl,
        "be" => Language::Be,
        "uk" => Language::Uk,
        "zh" => Language::ZhHans,
        "ja" => Language::Ja,
        "ko" => Language::Ko,
        _ => Language::En,
    }
}

#[cfg(test)]
mod tests {
    use super::{Language, resolve};

    #[test]
    fn a_missing_locale_entry_falls_back_to_english() {
        let lookup = |language: Language, key: &str| match (language, key) {
            (Language::En, "Only English") => Some("Only English"),
            _ => None,
        };
        assert_eq!(
            resolve(Language::Fr, "Only English", lookup),
            "Only English"
        );
    }

    #[test]
    fn a_key_missing_from_english_reads_as_itself() {
        let lookup = |_: Language, _: &str| None;
        assert_eq!(
            resolve(Language::Fr, "Not in the catalog", lookup),
            "Not in the catalog"
        );
    }
}
