use regex::Regex;
use tabtool_i18n::{LOCALES, Language, key_count, keys, language_from_system, tr, tr_in};

#[test]
fn the_catalog_has_the_translated_keys() {
    assert!(key_count() > 0, "the catalog is not empty");
    assert!(keys().any(|key| key == "Decimal"));
    assert!(keys().any(|key| key == "Saving… %lld%%"));
}

#[test]
fn every_key_exists_in_every_locale() {
    assert!(key_count() > 0, "the catalog is not empty");
    let mut problems = Vec::new();
    for key in keys() {
        for code in LOCALES {
            let language = Language::from_code(code).expect("every catalog locale is a language");
            match tr_in(language, key) {
                Some(value) if !value.is_empty() => {}
                _ => problems.push(format!("{code}: {key}")),
            }
        }
    }
    assert!(
        problems.is_empty(),
        "missing translations:\n{}",
        problems.join("\n")
    );
}

#[test]
fn every_translation_keeps_the_english_format_specifiers() {
    let mut problems = Vec::new();
    for key in keys() {
        for code in LOCALES {
            let language = Language::from_code(code).expect("every catalog locale is a language");
            let translation = tr_in(language, key).expect("every key has every translation");
            if !keeps_format_specifiers(translation, key) {
                problems.push(format!("{code}: {key} -> {translation}"));
            }
        }
    }
    assert!(
        problems.is_empty(),
        "format specifiers differ:\n{}",
        problems.join("\n")
    );
}

#[test]
fn runtime_strings_resolve_in_french() {
    assert_eq!(tr(Language::Fr, "Decimal"), "Décimal");
    assert_eq!(tr(Language::Fr, "Edit Cell"), "Modifier la cellule");
    assert_eq!(tr(Language::Fr, "Paste Data"), "Coller les données");
    for name in [
        "Paste Data",
        "Edit Cell",
        "Add Row",
        "Delete Rows",
        "Move Rows",
        "Add Column",
        "Delete Columns",
        "Move Column",
        "Move Columns",
        "Change Format",
        "Toggle Header",
        "Sort Rows",
        "Remove Sort",
        "Rename Column",
        "Revert to Saved",
    ] {
        assert_ne!(tr(Language::Fr, name), name, "{name} is translated");
    }
}

#[test]
fn system_and_unknown_languages_fall_back_to_english() {
    assert_eq!(tr(Language::System, "Cancel"), "Cancel");
    assert_eq!(tr(Language::En, "Cancel"), "Cancel");
    assert_eq!(tr(Language::De, "no such key"), "");
}

#[test]
fn the_language_list_is_system_first_and_unique() {
    assert_eq!(Language::ALL.len(), 12);
    assert_eq!(Language::ALL[0], Language::System);
    let mut codes: Vec<_> = Language::ALL
        .iter()
        .filter_map(|language| language.code())
        .collect();
    codes.sort_unstable();
    codes.dedup();
    assert_eq!(codes.len(), 11);
    assert_eq!(LOCALES.len(), 11);
    let ordered: Vec<_> = Language::ALL[1..]
        .iter()
        .map(|language| language.code().expect("every offered language has a code"))
        .collect();
    assert_eq!(ordered, LOCALES);
}

#[test]
fn only_system_has_no_native_name_and_native_names_are_not_translated() {
    for language in Language::ALL {
        assert_eq!(
            language.native_name().is_none(),
            language == Language::System
        );
    }
    assert_eq!(Language::En.native_name(), Some("English"));
    assert_eq!(Language::Fr.native_name(), Some("Français"));
    assert_eq!(Language::Es.native_name(), Some("Español"));
    assert_eq!(Language::De.native_name(), Some("Deutsch"));
    assert_eq!(Language::It.native_name(), Some("Italiano"));
    assert_eq!(Language::Pl.native_name(), Some("Polski"));
    assert_eq!(Language::Be.native_name(), Some("Беларуская"));
    assert_eq!(Language::Uk.native_name(), Some("Українська"));
    assert_eq!(Language::ZhHans.native_name(), Some("简体中文"));
    assert_eq!(Language::Ja.native_name(), Some("日本語"));
    assert_eq!(Language::Ko.native_name(), Some("한국어"));
}

#[test]
fn language_codes_round_trip() {
    for language in Language::ALL {
        assert_eq!(
            Language::from_persisted(language.persisted()),
            Some(language)
        );
    }
    assert_eq!(Language::ZhHans.persisted(), "zh-Hans");
    assert_eq!(Language::System.persisted(), "system");
    assert_eq!(Language::from_persisted("zh-Hans"), Some(Language::ZhHans));
    assert_eq!(Language::from_persisted("xx"), None);
}

#[test]
fn system_language_tags_map_to_the_catalog_language() {
    let strings = |values: &[&str]| {
        values
            .iter()
            .map(|value| value.to_string())
            .collect::<Vec<_>>()
    };

    assert_eq!(
        language_from_system(&strings(&["en-US", "fr"])),
        Language::En
    );
    assert_eq!(language_from_system(&strings(&["fr-FR"])), Language::Fr);
    assert_eq!(language_from_system(&strings(&["es-419"])), Language::Es);
    assert_eq!(language_from_system(&strings(&["de-DE"])), Language::De);
    assert_eq!(language_from_system(&strings(&["it-IT"])), Language::It);
    assert_eq!(language_from_system(&strings(&["pl-PL"])), Language::Pl);
    assert_eq!(language_from_system(&strings(&["be-BY"])), Language::Be);
    assert_eq!(language_from_system(&strings(&["uk-UA"])), Language::Uk);
    assert_eq!(language_from_system(&strings(&["ja-JP"])), Language::Ja);
    assert_eq!(language_from_system(&strings(&["ko-KR"])), Language::Ko);
    assert_eq!(
        language_from_system(&strings(&["zh-Hans"])),
        Language::ZhHans
    );
    assert_eq!(
        language_from_system(&strings(&["zh-Hans-CN"])),
        Language::ZhHans
    );
    assert_eq!(language_from_system(&strings(&["zh-CN"])), Language::ZhHans);
    assert_eq!(language_from_system(&strings(&["zh"])), Language::ZhHans);
    assert_eq!(
        language_from_system(&strings(&["ZH-hans", "en"])),
        Language::ZhHans
    );
    assert_eq!(language_from_system(&strings(&["ru-RU"])), Language::En);
    assert_eq!(language_from_system(&strings(&[])), Language::En);
}

/// Port of the Swift `keepsFormatSpecifiers`: without positional indexes the
/// specifiers keep the key's order, with them only the types are compared.
fn keeps_format_specifiers(translation: &str, key: &str) -> bool {
    let specifier = Regex::new(r"%(\d+\$)?([lh]*[@dDfsSuUxXcCp]|%)").expect("valid pattern");
    let found: Vec<_> = specifier.captures_iter(translation).collect();
    let types: Vec<&str> = found.iter().map(|capture| &capture[2]).collect();
    let positional = found.iter().any(|capture| capture.get(1).is_some());
    let key_types: Vec<&str> = specifier
        .captures_iter(key)
        .map(|capture| capture.get(2).expect("the type group matched").as_str())
        .collect();
    if positional {
        let mut types = types;
        let mut key_types = key_types;
        types.sort_unstable();
        key_types.sort_unstable();
        types == key_types
    } else {
        types == key_types
    }
}

#[test]
fn format_specifier_rules_ignore_positional_indexes() {
    assert!(keeps_format_specifiers("%1$@ %2$lld", "%lld %@"));
    assert!(keeps_format_specifiers("%2$lld %1$@", "%@ %lld"));
    assert!(!keeps_format_specifiers("%1$@", "%lld"));
    assert!(keeps_format_specifiers("%lld%% · %@", "%lld%% · %@"));
    assert!(!keeps_format_specifiers("%lld %@", "%@ %lld"));
    assert!(!keeps_format_specifiers("%@", "%lld"));
}
