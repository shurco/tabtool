use tabtool_platform::{DesktopPlatform, SystemInfo, decimal_separator, parse_locale};

#[test]
fn the_decimal_separator_comes_from_the_locale() {
    assert_eq!(decimal_separator("en_US"), ".");
    assert_eq!(decimal_separator("de_DE"), ",");
    assert_eq!(decimal_separator("fr_FR"), ",");
    assert_eq!(decimal_separator("en-US"), ".");
    assert_eq!(decimal_separator("de-DE"), ",");
}

#[test]
fn a_posix_tag_keeps_its_decimal_separator() {
    assert_eq!(decimal_separator("de_DE.UTF-8"), ",");
    assert_eq!(decimal_separator("fr_FR@euro"), ",");
}

#[test]
fn an_unknown_locale_keeps_the_dot() {
    assert_eq!(decimal_separator("not a locale"), ".");
    assert_eq!(decimal_separator(""), ".");
}

#[test]
fn the_system_locale_is_reported_as_a_catalog_like_tag() {
    let locale = DesktopPlatform.locale();
    assert!(!locale.contains('-'), "{locale} uses underscores");
}

#[test]
fn the_system_locale_implements_the_system_info_trait() {
    fn assert_system_info<T: SystemInfo>() {}
    assert_system_info::<DesktopPlatform>();
    let _ = DesktopPlatform.languages();
    let separator = DesktopPlatform.decimal_separator();
    assert!(separator == "." || separator == ",", "{separator}");
}

#[test]
fn an_underscore_system_tag_parses_as_its_icu_locale() {
    assert_eq!(parse_locale("de_DE").to_string(), "de-DE");
    assert_eq!(parse_locale("en-US").to_string(), "en-US");
}

#[test]
fn a_posix_tag_drops_its_encoding_and_modifier() {
    assert_eq!(parse_locale("de_DE.UTF-8").to_string(), "de-DE");
    assert_eq!(parse_locale("de_DE@euro").to_string(), "de-DE");
}

#[test]
fn an_unparsable_tag_falls_back_to_english() {
    assert_eq!(parse_locale("not a locale").to_string(), "en");
    assert_eq!(parse_locale("").to_string(), "en");
}
