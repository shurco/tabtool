//! System locale and decimal separator.

use icu_decimal::DecimalFormatter;
use icu_decimal::input::Decimal;
use icu_locale_core::Locale;

/// The system's preferred languages and number formatting.
pub trait SystemInfo {
    /// The preferred languages, most preferred first.
    fn languages(&self) -> Vec<String>;
    /// The system locale, for example `de_DE`.
    fn locale(&self) -> String;
    /// The locale's decimal separator, `","` or `"."`.
    fn decimal_separator(&self) -> String;
}

/// The system locale, for example `de_DE`.
#[cfg(not(target_os = "android"))]
pub fn system_locale() -> String {
    let tag = sys_locale::get_locale().unwrap_or_else(|| "en_US".to_string());
    normalize_tag(&tag)
}

/// The ICU locale of a platform tag such as `de_DE` or `de_DE.UTF-8`, or
/// `en` when the tag does not parse.
///
/// Platforms report underscore tags, which the ICU parser rejects, so the tag
/// is normalized and its underscores become hyphens first.
pub fn parse_locale(tag: &str) -> Locale {
    Locale::try_from_str(&normalize_tag(tag).replace('_', "-"))
        .unwrap_or_else(|_| Locale::try_from_str("en").expect("English is a valid ICU locale"))
}

/// The decimal separator of `locale`, `","` or `"."`.
///
/// Locales outside the two CSV decimal marks, or an unparsable tag, keep the
/// dot, which is the default new document format.
pub fn decimal_separator(locale: &str) -> String {
    let Ok(formatter) = DecimalFormatter::try_new(parse_locale(locale).into(), Default::default())
    else {
        return ".".to_string();
    };
    let mut number = Decimal::from(15);
    number.multiply_pow10(-1);
    let formatted = formatter.format_to_string(&number);
    if formatted.contains(',') {
        ",".to_string()
    } else {
        ".".to_string()
    }
}

/// Whether the system language makes the detector try GBK before CP1252.
///
/// This follows the first system language, not the interface override: a
/// Chinese system with an English UI still prefers GBK.
pub fn prefers_chinese_encoding(system_tags: &[String]) -> bool {
    tabtool_i18n::language_from_system(system_tags) == tabtool_i18n::Language::ZhHans
}

/// Removes an encoding suffix and turns the tag into an underscore locale.
pub(crate) fn normalize_tag(tag: &str) -> String {
    let tag = tag.split(['.', '@']).next().unwrap_or(tag);
    tag.replace('-', "_")
}

#[cfg(test)]
mod tests {
    use super::normalize_tag;

    #[test]
    fn locale_tags_drop_the_encoding_and_use_underscores() {
        assert_eq!(normalize_tag("de-DE"), "de_DE");
        assert_eq!(normalize_tag("en_US.UTF-8"), "en_US");
        assert_eq!(normalize_tag("sr_RS@latin"), "sr_RS");
    }
}
