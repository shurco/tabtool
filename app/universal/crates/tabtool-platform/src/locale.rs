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

/// The live system locale.
#[derive(Clone, Copy, Debug, Default)]
pub struct SystemLocale;

impl SystemInfo for SystemLocale {
    fn languages(&self) -> Vec<String> {
        sys_locale::get_locales().collect()
    }

    fn locale(&self) -> String {
        system_locale()
    }

    fn decimal_separator(&self) -> String {
        decimal_separator(&system_locale())
    }
}

/// The system locale, for example `de_DE`.
pub fn system_locale() -> String {
    let tag = sys_locale::get_locale().unwrap_or_else(|| "en_US".to_string());
    normalize_tag(&tag)
}

/// The decimal separator of `locale`, `","` or `"."`.
///
/// Locales outside the two CSV decimal marks, or an unparsable tag, keep the
/// dot, which is the default new document format.
pub fn decimal_separator(locale: &str) -> String {
    let tag = locale.replace('_', "-");
    let Ok(locale) = Locale::try_from_str(&tag) else {
        return ".".to_string();
    };
    let Ok(formatter) = DecimalFormatter::try_new(locale.into(), Default::default()) else {
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

/// Removes an encoding suffix and turns the tag into an underscore locale.
fn normalize_tag(tag: &str) -> String {
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
