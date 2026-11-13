//! Status-line text: counts, filters, format and large-file progress.

use tabtool_core::{CsvConfiguration, Encoding};
use tabtool_document::LargeStatus;
use tabtool_i18n::Language;
use tabtool_ui::document::{indexing_text, saving_text, status_summary};
use tabtool_ui::harness::kittest::Queryable;

use crate::support::{Fixture, MemoryStorage, loaded, untitled};

const EN: Language = Language::En;

fn configuration() -> CsvConfiguration {
    CsvConfiguration::default()
}

#[test]
fn a_rows_and_columns_summary_names_the_format() {
    let text = status_summary(
        EN,
        1_799,
        12,
        &configuration(),
        None,
        LargeStatus::None,
        ".",
    );

    assert_eq!(text, "1,799 rows × 12 columns · Unicode (UTF-8) · Comma");
}

#[test]
fn a_filtered_summary_shows_the_visible_and_total_counts() {
    let text = status_summary(
        EN,
        1_799,
        12,
        &configuration(),
        Some(42),
        LargeStatus::None,
        ".",
    );

    assert_eq!(
        text,
        "42 of 1,799 rows × 12 columns · Unicode (UTF-8) · Comma"
    );
}

#[test]
fn singular_rows_and_columns_use_the_singular_catalog_entries() {
    assert_eq!(
        status_summary(EN, 1, 1, &configuration(), None, LargeStatus::None, "."),
        "1 row × 1 column · Unicode (UTF-8) · Comma"
    );
}

#[test]
fn indexing_reports_the_fraction_and_rows_so_far() {
    assert_eq!(
        indexing_text(EN, 0.426, 1_234, "."),
        "Indexing… 42% · 1,234 rows so far"
    );
    assert_eq!(
        indexing_text(EN, 1.5, 7, "."),
        "Indexing… 100% · 7 rows so far"
    );
}

#[test]
fn saving_reports_the_fraction() {
    assert_eq!(saving_text(EN, 0.125), "Saving… 12%");
    assert_eq!(saving_text(EN, -1.0), "Saving… 0%");
}

#[test]
fn the_status_line_uses_the_document_separator_name() {
    let text = status_summary(
        EN,
        2,
        3,
        &CsvConfiguration {
            encoding: Encoding::Utf8,
            column_separator: "\t".to_owned(),
            ..configuration()
        },
        None,
        LargeStatus::None,
        ".",
    );

    assert_eq!(text, "2 rows × 3 columns · Unicode (UTF-8) · Tab");
}

#[test]
fn the_rendered_status_line_matches_the_summary() {
    let storage = MemoryStorage::new(b"a,b\n1,2\n");
    let mut fixture = Fixture::new(
        loaded(&storage, configuration()),
        MemoryStorage::new(b"a,b\n1,2\n"),
        Default::default(),
    );
    fixture.run();

    assert!(
        fixture
            .query_by_label("2 rows × 2 columns · Unicode (UTF-8) · Comma")
            .is_some()
    );
}

#[test]
fn an_untitled_document_shows_its_row_count() {
    let mut fixture = Fixture::new(
        untitled(configuration()),
        MemoryStorage::new(b""),
        Default::default(),
    );
    fixture.run();

    assert!(
        fixture
            .query_by_label("1 row × 3 columns · Unicode (UTF-8) · Comma")
            .is_some()
    );
}
