//! Ports of the Apple `CSVFormatDetectorTests` suite and the Review Focus
//! cases.

use tabtool_core::{CsvConfiguration, CsvFormatDetector, Encoding};
use tabtool_testdata::fixture;

/// Detects the format of a fixture in the `Heuristic` folder.
fn detect(name: &str) -> CsvConfiguration {
    detect_bytes(&fixture(&format!("Heuristic/{name}.csv")), false)
}

fn detect_bytes(data: &[u8], prefers_chinese_encoding: bool) -> CsvConfiguration {
    CsvFormatDetector::new(data, prefers_chinese_encoding).detect()
}

/// A configuration with every field spelled out.
fn expected(
    encoding: Encoding,
    column_separator: &str,
    quote_character: &str,
    escape_character: &str,
    decimal_mark: &str,
    first_row_as_header: bool,
) -> CsvConfiguration {
    CsvConfiguration {
        encoding,
        column_separator: column_separator.to_owned(),
        quote_character: quote_character.to_owned(),
        escape_character: escape_character.to_owned(),
        decimal_mark: decimal_mark.to_owned(),
        first_row_as_header,
    }
}

/// The eleven designed configurations, in fixture numbering order.
#[test]
fn detects_expected_formats() {
    let cases = [
        (
            "heuristic-config1",
            expected(Encoding::Utf8, ",", "\"", "\"", ".", true),
        ),
        (
            "heuristic-config2",
            expected(Encoding::Utf8, ";", "\"", "\"", ",", true),
        ),
        (
            "heuristic-config3",
            expected(Encoding::Utf8, "\t", "\"", "\"", ".", true),
        ),
        (
            "heuristic-config4",
            expected(Encoding::Utf8, "\t", "\"", "\"", ",", true),
        ),
        (
            "heuristic-config5",
            expected(Encoding::Utf8, ";", "\"", "\\", ",", true),
        ),
        (
            "heuristic-config6",
            expected(Encoding::Utf8, ",", "", "\"", ".", true),
        ),
        (
            "heuristic-config7",
            expected(Encoding::Utf8, ";", "", "\"", ",", true),
        ),
        (
            "heuristic-config8",
            expected(Encoding::Utf8, "\t", "", "\"", ".", true),
        ),
        (
            "heuristic-config9",
            expected(Encoding::Utf8, "\t", "", "\"", ",", true),
        ),
        (
            "heuristic-config10",
            expected(Encoding::Utf8, ",", "\"", "\\", ".", true),
        ),
        (
            "heuristic-config11",
            expected(Encoding::Utf8, ",", "\"", "\"", ",", true),
        ),
    ];

    for (name, configuration) in cases {
        assert_eq!(detect(name), configuration, "{name}");
    }
}

/// The Swift `peopleAndPlacesFixturesDetectCommaHeader` cases.
#[test]
fn people_and_places_fixtures_detect_comma_header() {
    let configuration = expected(Encoding::Utf8, ",", "\"", "\"", ".", true);
    for name in [
        "heuristic-comma-separated-people-1",
        "heuristic-comma-separated-people-2",
        "heuristic-comma-separated-places",
    ] {
        assert_eq!(detect(name), configuration, "{name}");
    }
}

#[test]
fn empty_first_row_cell_means_no_header() {
    let configuration = detect_bytes(b",name\n1,2", false);
    assert_eq!(configuration.column_separator, ",");
    assert!(!configuration.first_row_as_header);
}

#[test]
fn malformed_quoted_data_falls_back_to_unquoted_detection() {
    let data = fixture("Reading/tooMany-quotes-atEnd.csv");
    assert_eq!(
        detect_bytes(&data, false),
        expected(Encoding::Utf8, ",", "", "\"", ".", true)
    );
}

#[test]
fn first_row_number_means_no_header() {
    assert_eq!(
        detect("heuristic-first-row-with-number"),
        expected(Encoding::Utf8, ",", "\"", "\"", ".", false)
    );
}

#[test]
fn shorter_first_row_means_no_header() {
    assert_eq!(
        detect("heuristic-first-row-one-row-shorter"),
        expected(Encoding::Utf8, ",", "\"", "\"", ".", false)
    );
}

#[test]
fn longer_first_row_means_no_header() {
    assert_eq!(
        detect("heuristic-first-row-longer"),
        expected(Encoding::Utf8, ",", "\"", "\"", ".", false)
    );
}

#[test]
fn cp1252_is_detected() {
    assert_eq!(
        detect("heuristic-no-utf-encoding"),
        expected(Encoding::Windows1252, ",", "\"", "\"", ".", false)
    );
}

#[test]
fn gbk_is_preferred_for_chinese() {
    let data = fixture("Heuristic/heuristic-gbk.csv");

    assert_eq!(
        detect_bytes(&data, false),
        expected(Encoding::Windows1252, ",", "\"", "\"", ".", true)
    );
    assert_eq!(
        detect_bytes(&data, true),
        expected(Encoding::Gbk, ",", "\"", "\"", ".", true)
    );
}

/// GBK moves ahead of CP1252 only when Chinese is preferred, for both a
/// Chinese fixture and Western bytes that are valid GBK.
#[test]
fn gbk_comes_before_cp1252_only_when_chinese_is_preferred() {
    for (name, header) in [
        ("heuristic-gbk", true),
        ("heuristic-no-utf-encoding", false),
    ] {
        let data = fixture(&format!("Heuristic/{name}.csv"));
        assert_eq!(
            detect_bytes(&data, false).encoding,
            Encoding::Windows1252,
            "{name}"
        );
        assert_eq!(
            detect_bytes(&data, true),
            expected(Encoding::Gbk, ",", "\"", "\"", ".", header),
            "{name}"
        );
    }
}

#[test]
fn macroman_tab_sample_is_detected() {
    assert_eq!(
        detect("macroman-tab-sample"),
        expected(Encoding::MacRoman, "\t", "\"", "\"", ".", true)
    );
}

/// Two candidates score the same; the earlier quoted comma configuration wins.
#[test]
fn earlier_configuration_wins_ties() {
    assert_eq!(
        detect_bytes(b"a,b\nc,d\n", false),
        expected(Encoding::Utf8, ",", "\"", "\"", ".", true)
    );
}

/// Empty data scores every configuration the same and decodes as UTF-8.
#[test]
fn empty_data_yields_default_configuration() {
    assert_eq!(detect_bytes(b"", false), CsvConfiguration::default());
    assert_eq!(detect_bytes(b"", true), CsvConfiguration::default());
}

/// Invalid bytes for the earlier encodings are skipped, not panicked on.
#[test]
fn invalid_bytes_still_return_a_configuration() {
    let configuration = detect_bytes(&[0xFF, 0xFE, 0x81, 0x00], true);
    assert_eq!(configuration.encoding, Encoding::MacRoman);
}
