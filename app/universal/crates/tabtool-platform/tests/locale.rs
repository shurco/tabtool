use tabtool_platform::{SystemInfo, SystemLocale, decimal_separator};

#[test]
fn the_decimal_separator_comes_from_the_locale() {
    assert_eq!(decimal_separator("en_US"), ".");
    assert_eq!(decimal_separator("de_DE"), ",");
    assert_eq!(decimal_separator("fr_FR"), ",");
    assert_eq!(decimal_separator("en-US"), ".");
    assert_eq!(decimal_separator("de-DE"), ",");
}

#[test]
fn an_unknown_locale_keeps_the_dot() {
    assert_eq!(decimal_separator("not a locale"), ".");
    assert_eq!(decimal_separator(""), ".");
}

#[test]
fn the_system_locale_is_reported_as_a_catalog_like_tag() {
    let locale = SystemLocale.locale();
    assert!(!locale.contains('-'), "{locale} uses underscores");
}

#[test]
fn the_system_locale_implements_the_system_info_trait() {
    fn assert_system_info<T: SystemInfo>() {}
    assert_system_info::<SystemLocale>();
    let _ = SystemLocale.languages();
    let separator = SystemLocale.decimal_separator();
    assert!(separator == "." || separator == ",", "{separator}");
}
