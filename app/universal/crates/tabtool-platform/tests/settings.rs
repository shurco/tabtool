use std::fs;
use std::path::{Path, PathBuf};

use serde_json::json;
use tabtool_core::{CsvConfiguration, Encoding};
use tabtool_i18n::Language;
use tabtool_platform::{
    LARGE_FILE_THRESHOLDS_MIB, QuoteStyle, RECENT_DOCUMENTS_LIMIT, ROW_HEIGHTS, Settings, keys,
    settings_path,
};
use tempfile::TempDir;

/// Foundation's `String.Encoding` raw values, the persisted encoding IDs.
const FOUNDATION_IDS: [(Encoding, i64); 18] = [
    (Encoding::Utf8, 4),
    (Encoding::MacRoman, 30),
    (Encoding::Windows1252, 12),
    (Encoding::Gbk, 2_147_485_234),
    (Encoding::IsoLatin2, 9),
    (Encoding::Windows1250, 15),
    (Encoding::Windows1251, 11),
    (Encoding::Windows1253, 13),
    (Encoding::Windows1254, 14),
    (Encoding::EucJp, 3),
    (Encoding::ShiftJis, 8),
    (Encoding::Iso2022Jp, 21),
    (Encoding::Utf16, 10),
    (Encoding::Utf16Be, 2_415_919_360),
    (Encoding::Utf16Le, 2_483_028_224),
    (Encoding::Utf32, 2_348_810_496),
    (Encoding::Utf32Be, 2_550_137_088),
    (Encoding::Utf32Le, 2_617_245_952),
];

fn path_in(dir: &TempDir) -> PathBuf {
    dir.path().join("settings.json")
}

fn write_json(path: &Path, value: serde_json::Value) {
    fs::write(path, serde_json::to_vec_pretty(&value).expect("json")).expect("write settings");
}

#[test]
fn key_strings_are_persisted_data() {
    assert_eq!(keys::SHOW_FORMAT_BAR, "showFormatBar");
    assert_eq!(keys::APP_LANGUAGE, "appLanguage");
    assert_eq!(keys::NEW_DOCUMENT_ENCODING, "newDocumentEncoding");
    assert_eq!(keys::NEW_DOCUMENT_SEPARATOR, "newDocumentSeparator");
    assert_eq!(keys::NEW_DOCUMENT_QUOTE_STYLE, "newDocumentQuoteStyle");
    assert_eq!(keys::NEW_DOCUMENT_DECIMAL_MARK, "newDocumentDecimalMark");
    assert_eq!(
        keys::NEW_DOCUMENT_FIRST_ROW_AS_HEADER,
        "newDocumentFirstRowAsHeader"
    );
    assert_eq!(keys::GRID_ALTERNATING_ROWS, "gridAlternatingRows");
    assert_eq!(keys::GRID_ROW_HEIGHT, "gridRowHeight");
    assert_eq!(keys::GRID_SHOWS_ROW_NUMBERS, "gridShowsRowNumbers");
    assert_eq!(keys::LARGE_FILE_THRESHOLD_MIB, "largeFileThresholdMiB");
    assert_eq!(keys::REVIEW_FIRST_LAUNCH, "reviewFirstLaunch");
    assert_eq!(keys::REVIEW_ACTIVE_DAYS, "reviewActiveDays");
    assert_eq!(keys::REVIEW_LAST_ACTIVE_DAY, "reviewLastActiveDay");
    assert_eq!(keys::REVIEW_ENGAGEMENTS, "reviewEngagements");
    assert_eq!(
        keys::REVIEW_LAST_PROMPTED_VERSION,
        "reviewLastPromptedVersion"
    );
    assert_eq!(keys::REVIEW_LAST_PROMPTED_AT, "reviewLastPromptedAt");
    assert_eq!(keys::RECENT_DOCUMENTS, "recentDocuments");
}

#[test]
fn empty_defaults_give_the_documented_defaults() {
    let settings = Settings::default();
    assert_eq!(
        settings.new_document_configuration(),
        CsvConfiguration::default()
    );
    assert_eq!(settings.large_file_threshold_bytes(), 100 * 1_048_576);
    assert!(settings.show_format_bar);
    assert_eq!(settings.app_language, Language::System);
    assert_eq!(settings.grid_row_height, 24.0);
    assert!(!settings.grid_alternating_rows);
    assert!(settings.grid_shows_row_numbers);
    assert!(settings.new_document_decimal_mark == ".");
    assert_eq!(settings.new_document_quote_style, QuoteStyle::Doubled);
    assert!(!settings.new_document_first_row_as_header);
    assert!(settings.recent_documents.is_empty());
}

#[test]
fn missing_file_gives_the_defaults() {
    let dir = TempDir::new().expect("temp dir");
    assert_eq!(Settings::load(&path_in(&dir)), Settings::default());
}

#[test]
fn malformed_file_gives_the_defaults() {
    let dir = TempDir::new().expect("temp dir");
    let path = path_in(&dir);
    fs::write(&path, b"not json").expect("write");
    assert_eq!(Settings::load(&path), Settings::default());
    fs::write(&path, b"[]").expect("write");
    assert_eq!(Settings::load(&path), Settings::default());
}

#[test]
fn stored_values_are_read() {
    let dir = TempDir::new().expect("temp dir");
    let path = path_in(&dir);
    write_json(
        &path,
        json!({
            keys::SHOW_FORMAT_BAR: false,
            keys::APP_LANGUAGE: "fr",
            keys::NEW_DOCUMENT_ENCODING: 11,
            keys::NEW_DOCUMENT_SEPARATOR: ";",
            keys::NEW_DOCUMENT_QUOTE_STYLE: "backslash",
            keys::NEW_DOCUMENT_DECIMAL_MARK: ",",
            keys::NEW_DOCUMENT_FIRST_ROW_AS_HEADER: true,
            keys::GRID_ALTERNATING_ROWS: true,
            keys::GRID_ROW_HEIGHT: 32,
            keys::GRID_SHOWS_ROW_NUMBERS: false,
            keys::LARGE_FILE_THRESHOLD_MIB: 250,
            keys::RECENT_DOCUMENTS: ["/tmp/a.csv", "/tmp/b.csv"],
            keys::REVIEW_FIRST_LAUNCH: "2026-01-02T03:04:05Z",
            keys::REVIEW_ACTIVE_DAYS: 4,
            keys::REVIEW_LAST_ACTIVE_DAY: "2026-01-05T00:00:00Z",
            keys::REVIEW_ENGAGEMENTS: 6,
            keys::REVIEW_LAST_PROMPTED_VERSION: "1.0.1",
            keys::REVIEW_LAST_PROMPTED_AT: "2026-01-06T00:00:00Z"
        }),
    );

    let settings = Settings::load(&path);
    assert!(!settings.show_format_bar);
    assert_eq!(settings.app_language, Language::Fr);
    assert_eq!(settings.new_document_encoding, Encoding::Windows1251);
    assert_eq!(settings.new_document_separator, ";");
    assert_eq!(settings.new_document_quote_style, QuoteStyle::Backslash);
    assert_eq!(settings.new_document_decimal_mark, ",");
    assert!(settings.new_document_first_row_as_header);
    assert!(settings.grid_alternating_rows);
    assert_eq!(settings.grid_row_height, 32.0);
    assert!(!settings.grid_shows_row_numbers);
    assert_eq!(settings.large_file_threshold_mib, 250);
    assert_eq!(
        settings.recent_documents,
        vec![PathBuf::from("/tmp/a.csv"), PathBuf::from("/tmp/b.csv")]
    );
    assert_eq!(settings.review_active_days, 4);
    assert_eq!(settings.review_engagements, 6);

    let configuration = settings.new_document_configuration();
    assert_eq!(configuration.encoding, Encoding::Windows1251);
    assert_eq!(configuration.column_separator, ";");
    assert_eq!(configuration.quote_character, "\"");
    assert_eq!(configuration.escape_character, "\\");
    assert_eq!(configuration.decimal_mark, ",");
    assert!(configuration.first_row_as_header);
    assert_eq!(settings.large_file_threshold_bytes(), 250 * 1_048_576);
}

#[test]
fn unknown_or_mistyped_values_fall_back_to_defaults() {
    let dir = TempDir::new().expect("temp dir");
    let path = path_in(&dir);
    write_json(
        &path,
        json!({
            keys::SHOW_FORMAT_BAR: "yes",
            keys::APP_LANGUAGE: "xx",
            keys::NEW_DOCUMENT_ENCODING: 999_999,
            keys::NEW_DOCUMENT_SEPARATOR: "x",
            keys::NEW_DOCUMENT_QUOTE_STYLE: "x",
            keys::NEW_DOCUMENT_DECIMAL_MARK: "x",
            keys::NEW_DOCUMENT_FIRST_ROW_AS_HEADER: 1,
            keys::GRID_ALTERNATING_ROWS: "true",
            keys::GRID_ROW_HEIGHT: 7,
            keys::GRID_SHOWS_ROW_NUMBERS: "no",
            keys::LARGE_FILE_THRESHOLD_MIB: 7,
            keys::RECENT_DOCUMENTS: ["/tmp/a.csv", 42],
            keys::REVIEW_ACTIVE_DAYS: -1
        }),
    );
    assert_eq!(Settings::load(&path), Settings::default());
}

#[test]
fn invalid_thresholds_fall_back_to_the_default() {
    for invalid in [0, -5, 7, 99, 1025] {
        let dir = TempDir::new().expect("temp dir");
        let path = path_in(&dir);
        write_json(&path, json!({ keys::LARGE_FILE_THRESHOLD_MIB: invalid }));
        assert_eq!(
            Settings::load(&path).large_file_threshold_bytes(),
            100 * 1_048_576,
            "{invalid} falls back"
        );
    }
    for valid in LARGE_FILE_THRESHOLDS_MIB {
        let dir = TempDir::new().expect("temp dir");
        let path = path_in(&dir);
        write_json(&path, json!({ keys::LARGE_FILE_THRESHOLD_MIB: valid }));
        assert_eq!(
            Settings::load(&path).large_file_threshold_bytes(),
            u64::from(valid) * 1_048_576
        );
    }
}

#[test]
fn row_heights_the_settings_offer_are_kept() {
    for height in ROW_HEIGHTS {
        let settings = Settings {
            grid_row_height: height,
            ..Settings::default()
        };
        assert_eq!(settings.row_height(), height);
    }
}

#[test]
fn other_row_heights_fall_back_to_the_default() {
    for invalid in [0.0, 7.0, 99.0, -24.0] {
        let settings = Settings {
            grid_row_height: invalid,
            ..Settings::default()
        };
        assert_eq!(settings.row_height(), 24.0);
    }
    let dir = TempDir::new().expect("temp dir");
    let path = path_in(&dir);
    write_json(&path, json!({ keys::GRID_ROW_HEIGHT: 99 }));
    assert_eq!(Settings::load(&path).row_height(), 24.0);
}

#[test]
fn defaults_are_among_the_options_the_settings_offer() {
    assert!(LARGE_FILE_THRESHOLDS_MIB.contains(&100));
    assert!(ROW_HEIGHTS.contains(&24.0));
    assert_eq!(RECENT_DOCUMENTS_LIMIT, 10);
}

#[test]
fn alternating_rows_are_off_by_default() {
    assert!(!Settings::default().grid_alternating_rows);
}

#[test]
fn settings_round_trip_through_the_file() {
    let dir = TempDir::new().expect("temp dir");
    let path = path_in(&dir);
    let settings = Settings {
        show_format_bar: false,
        app_language: Language::ZhHans,
        new_document_encoding: Encoding::Windows1252,
        new_document_separator: "\t".to_string(),
        new_document_quote_style: QuoteStyle::None,
        new_document_decimal_mark: ",".to_string(),
        new_document_first_row_as_header: true,
        grid_alternating_rows: true,
        grid_row_height: 20.0,
        grid_shows_row_numbers: false,
        large_file_threshold_mib: 500,
        recent_documents: vec![PathBuf::from("/tmp/one.csv"), PathBuf::from("/tmp/two.csv")],
        review_first_launch: Some("2026-01-02T03:04:05Z".to_string()),
        review_active_days: 3,
        review_last_active_day: Some("2026-01-04T00:00:00Z".to_string()),
        review_engagements: 8,
        review_last_prompted_version: Some("1.0.1".to_string()),
        review_last_prompted_at: Some("2026-01-05T00:00:00Z".to_string()),
    };
    settings.save(&path).expect("save");
    assert_eq!(Settings::load(&path), settings);
}

#[test]
fn saving_overwrites_and_creates_the_parent_directory() {
    let dir = TempDir::new().expect("temp dir");
    let path = dir.path().join("nested").join("settings.json");
    Settings {
        grid_row_height: 32.0,
        ..Settings::default()
    }
    .save(&path)
    .expect("first save");
    Settings {
        grid_row_height: 20.0,
        ..Settings::default()
    }
    .save(&path)
    .expect("second save");
    assert_eq!(Settings::load(&path).grid_row_height, 20.0);
    let entries: Vec<_> = fs::read_dir(dir.path().join("nested"))
        .expect("read the settings directory")
        .map(|entry| entry.expect("entry").file_name())
        .collect();
    assert_eq!(entries, vec![std::ffi::OsString::from("settings.json")]);
}

#[test]
fn saved_json_uses_the_persisted_keys_and_foundation_encoding_ids() {
    for (encoding, id) in FOUNDATION_IDS {
        let dir = TempDir::new().expect("temp dir");
        let path = path_in(&dir);
        Settings {
            new_document_encoding: encoding,
            ..Settings::default()
        }
        .save(&path)
        .expect("save");
        let value: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).expect("read")).expect("json");
        assert_eq!(
            value[keys::NEW_DOCUMENT_ENCODING],
            id,
            "{encoding:?} keeps Foundation's raw value"
        );
        assert_eq!(Settings::load(&path).new_document_encoding, encoding);
    }
}

#[test]
fn saved_json_uses_the_app_language_code_and_the_recent_documents_key() {
    let dir = TempDir::new().expect("temp dir");
    let path = path_in(&dir);
    Settings {
        app_language: Language::De,
        recent_documents: vec![PathBuf::from("/tmp/recent.csv")],
        ..Settings::default()
    }
    .save(&path)
    .expect("save");
    let value: serde_json::Value =
        serde_json::from_slice(&fs::read(&path).expect("read")).expect("json");
    assert_eq!(value[keys::APP_LANGUAGE], "de");
    assert_eq!(value[keys::RECENT_DOCUMENTS][0], "/tmp/recent.csv");
    assert_eq!(value[keys::NEW_DOCUMENT_QUOTE_STYLE], "doubled");
}

#[test]
fn recent_documents_are_capped_at_ten() {
    let paths: Vec<String> = (0..12).map(|index| format!("/tmp/{index}.csv")).collect();
    let dir = TempDir::new().expect("temp dir");
    let path = path_in(&dir);
    write_json(&path, json!({ keys::RECENT_DOCUMENTS: paths }));
    let loaded = Settings::load(&path);
    assert_eq!(loaded.recent_documents.len(), 10);
    assert_eq!(loaded.recent_documents[0], PathBuf::from("/tmp/0.csv"));

    let settings = Settings {
        recent_documents: (0..12)
            .map(|index| PathBuf::from(format!("/tmp/{index}.csv")))
            .collect(),
        ..Settings::default()
    };
    settings.save(&path).expect("save");
    assert_eq!(Settings::load(&path).recent_documents.len(), 10);
}

#[test]
fn quote_styles_keep_the_swift_values_and_characters() {
    assert_eq!(QuoteStyle::Doubled.persisted(), "doubled");
    assert_eq!(QuoteStyle::Backslash.persisted(), "backslash");
    assert_eq!(QuoteStyle::None.persisted(), "none");
    assert_eq!(QuoteStyle::Doubled.quote_character(), "\"");
    assert_eq!(QuoteStyle::Doubled.escape_character(), "\"");
    assert_eq!(QuoteStyle::Backslash.quote_character(), "\"");
    assert_eq!(QuoteStyle::Backslash.escape_character(), "\\");
    assert_eq!(QuoteStyle::None.quote_character(), "");
    assert_eq!(QuoteStyle::None.escape_character(), "");
    for style in QuoteStyle::ALL {
        assert_eq!(QuoteStyle::from_persisted(style.persisted()), Some(style));
    }
    assert_eq!(
        QuoteStyle::from_persisted("doubled"),
        Some(QuoteStyle::Doubled)
    );
}

#[test]
fn the_settings_path_is_in_the_platform_config_directory() {
    let path = settings_path().expect("the platform has a config directory");
    assert_eq!(
        path.file_name().and_then(|name| name.to_str()),
        Some("settings.json")
    );
    assert!(
        path.to_string_lossy().to_lowercase().contains("tabtool"),
        "{} is in the TabTool directory",
        path.display()
    );
}
