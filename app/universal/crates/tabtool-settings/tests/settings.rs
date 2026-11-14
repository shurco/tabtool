use tabtool_core::Encoding;
use tabtool_i18n::Language;
use tabtool_settings::{
    LARGE_FILE_THRESHOLDS_MIB, MOBILE_DEFAULT_ROW_HEIGHT, MOBILE_ROW_HEIGHTS, QuoteStyle,
    ROW_HEIGHTS, Settings, VALID_ROW_HEIGHTS, keys, valid_mobile_row_height, valid_row_height,
};

#[test]
fn defaults_are_the_documented_values() {
    let settings = Settings::default();
    assert!(settings.show_format_bar);
    assert_eq!(settings.app_language, Language::System);
    assert_eq!(settings.new_document_encoding, Encoding::Utf8);
    assert_eq!(settings.new_document_separator, ",");
    assert_eq!(settings.new_document_quote_style, QuoteStyle::Doubled);
    assert_eq!(settings.new_document_decimal_mark, ".");
    assert!(!settings.new_document_first_row_as_header);
    assert!(!settings.grid_alternating_rows);
    assert_eq!(settings.grid_row_height, 24.0);
    assert!(settings.grid_shows_row_numbers);
    assert_eq!(settings.large_file_threshold_mib, 100);
    assert!(settings.recent_documents.is_empty());
}

#[test]
fn offered_values_are_the_documented_sets() {
    assert_eq!(ROW_HEIGHTS, [20.0, 24.0, 32.0]);
    assert_eq!(MOBILE_ROW_HEIGHTS, [32.0, 44.0, 56.0]);
    assert_eq!(VALID_ROW_HEIGHTS, [20.0, 24.0, 32.0, 44.0, 56.0]);
    assert_eq!(LARGE_FILE_THRESHOLDS_MIB, [25, 50, 100, 250, 500, 1024]);
    assert!(LARGE_FILE_THRESHOLDS_MIB.contains(&100));
    assert!(ROW_HEIGHTS.contains(&24.0));
    assert!(MOBILE_ROW_HEIGHTS.contains(&MOBILE_DEFAULT_ROW_HEIGHT));
}

#[test]
fn keys_are_the_persisted_apple_names() {
    assert_eq!(keys::SHOW_FORMAT_BAR, "showFormatBar");
    assert_eq!(keys::APP_LANGUAGE, "appLanguage");
    assert_eq!(keys::NEW_DOCUMENT_ENCODING, "newDocumentEncoding");
    assert_eq!(keys::GRID_ROW_HEIGHT, "gridRowHeight");
    assert_eq!(keys::LARGE_FILE_THRESHOLD_MIB, "largeFileThresholdMiB");
    assert_eq!(keys::RECENT_DOCUMENTS, "recentDocuments");
}

#[test]
fn a_new_document_uses_every_format_default() {
    let settings = Settings {
        new_document_encoding: Encoding::Windows1252,
        new_document_separator: "\t".to_owned(),
        new_document_quote_style: QuoteStyle::None,
        new_document_decimal_mark: ",".to_owned(),
        new_document_first_row_as_header: true,
        ..Settings::default()
    };

    let configuration = settings.new_document_configuration();
    assert_eq!(configuration.encoding, Encoding::Windows1252);
    assert_eq!(configuration.column_separator, "\t");
    assert!(configuration.quote_character.is_empty());
    assert!(configuration.escape_character.is_empty());
    assert_eq!(configuration.decimal_mark, ",");
    assert!(configuration.first_row_as_header);
}

#[test]
fn thresholds_are_mebibytes_and_row_heights_fall_back() {
    let settings = Settings {
        large_file_threshold_mib: 250,
        grid_row_height: 32.0,
        ..Settings::default()
    };
    assert_eq!(settings.large_file_threshold_bytes(), 250 * 1_048_576);
    assert_eq!(settings.row_height(), 32.0);

    for invalid in [0.0, 7.0, 99.0, -24.0] {
        assert_eq!(valid_row_height(invalid), 24.0);
    }
    for height in VALID_ROW_HEIGHTS {
        assert_eq!(valid_row_height(height), height);
    }
    for height in MOBILE_ROW_HEIGHTS {
        assert_eq!(valid_mobile_row_height(height), height);
    }
    assert_eq!(valid_mobile_row_height(24.0), MOBILE_DEFAULT_ROW_HEIGHT);
    let mobile = Settings {
        grid_row_height: 56.0,
        ..Settings::default()
    };
    assert_eq!(mobile.row_height(), 24.0);
    assert_eq!(mobile.mobile_row_height(), 56.0);
}

#[test]
fn quote_styles_round_trip_and_apply() {
    for style in QuoteStyle::ALL {
        assert_eq!(QuoteStyle::from_persisted(style.persisted()), Some(style));
    }
    assert_eq!(QuoteStyle::None.quote_character(), "");
    assert_eq!(QuoteStyle::Backslash.escape_character(), "\\");
    assert_eq!(QuoteStyle::Doubled.escape_character(), "\"");
}
