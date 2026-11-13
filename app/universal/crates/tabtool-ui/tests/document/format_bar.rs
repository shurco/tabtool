//! Format-bar values, changes, re-read confirmation and failure reporting.

use egui::accesskit::Role;
use tabtool_core::CsvConfiguration;
use tabtool_ui::document::{DocumentAction, LargeUiAction};
use tabtool_ui::harness::kittest::{NodeT, Queryable};

use crate::support::{Fixture, MemoryStorage, loaded, read_failed, untitled};

fn configuration() -> CsvConfiguration {
    CsvConfiguration::default()
}

fn summary(fixture: &Fixture) -> Option<String> {
    fixture.with_document(|document| {
        document.row_source().map(|source| {
            tabtool_ui::document::status_summary(
                tabtool_i18n::Language::En,
                source.row_count(),
                source.column_count(),
                document.configuration(),
                document.row_filter().visible_rows.as_ref().map(Vec::len),
                document.large_status(),
                ".",
            )
        })
    })
}

#[test]
fn the_format_bar_shows_the_current_format_and_status() {
    let mut fixture = Fixture::new(
        untitled(configuration()),
        MemoryStorage::new(b""),
        Default::default(),
    );
    fixture.run();

    assert_eq!(
        fixture
            .get_by_role_and_label(Role::ComboBox, "Encoding")
            .value()
            .as_deref(),
        Some("Unicode (UTF-8)")
    );
    assert_eq!(
        fixture
            .get_by_role_and_label(Role::ComboBox, "Separator")
            .value()
            .as_deref(),
        Some("Comma (,)")
    );
    assert_eq!(
        fixture
            .get_by_role_and_label(Role::ComboBox, "Quote")
            .value()
            .as_deref(),
        Some("Doubled")
    );
    assert_eq!(
        fixture
            .get_by_role_and_label(Role::ComboBox, "Decimal")
            .value()
            .as_deref(),
        Some("Dot (.)")
    );
    assert!(
        !fixture
            .get_by_role_and_label(Role::CheckBox, "Header")
            .accesskit_node()
            .is_disabled()
    );

    assert_eq!(
        summary(&fixture).as_deref(),
        Some("1 row × 3 columns · Unicode (UTF-8) · Comma")
    );
}

#[test]
fn an_untitled_separator_change_updates_the_configuration_in_place() {
    let mut fixture = Fixture::new(
        untitled(configuration()),
        MemoryStorage::new(b""),
        Default::default(),
    );
    fixture.run();

    fixture
        .get_by_role_and_label(Role::ComboBox, "Separator")
        .click();
    fixture.run();
    fixture.get_by_label("Semicolon (;)").click();
    fixture.settle();

    assert_eq!(
        fixture.with_document(|document| document.configuration().column_separator.clone()),
        ";"
    );
    assert_eq!(
        summary(&fixture).as_deref(),
        Some("1 row × 3 columns · Unicode (UTF-8) · Semicolon")
    );
}

#[test]
fn toggling_the_header_moves_the_first_row_across_the_boundary() {
    let mut fixture = Fixture::new(
        untitled(configuration()),
        MemoryStorage::new(b""),
        Default::default(),
    );
    fixture.run();

    fixture
        .get_by_role_and_label(Role::CheckBox, "Header")
        .click();
    fixture.settle();
    assert_eq!(
        summary(&fixture).as_deref(),
        Some("0 rows × 3 columns · Unicode (UTF-8) · Comma")
    );

    fixture
        .get_by_role_and_label(Role::CheckBox, "Header")
        .click();
    fixture.settle();
    assert_eq!(
        summary(&fixture).as_deref(),
        Some("1 row × 3 columns · Unicode (UTF-8) · Comma")
    );
}

#[test]
fn the_decimal_mark_is_disabled_in_large_mode() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let path = directory.path().join("large.csv");
    std::fs::write(&path, b"a,b\n1.5,2\n").expect("fixture file is written");
    let document = crate::support::large_ready(&path);
    let mut fixture = Fixture::new(document, MemoryStorage::new(b""), Default::default());
    fixture.run();

    assert!(
        fixture
            .get_by_role_and_label(Role::ComboBox, "Decimal")
            .accesskit_node()
            .is_disabled()
    );
}

#[test]
fn detect_automatically_needs_a_file_path() {
    let mut fixture = Fixture::new(
        untitled(configuration()),
        MemoryStorage::new(b""),
        Default::default(),
    );
    fixture.run();
    assert!(
        fixture
            .get_by_label("Detect Format Automatically")
            .accesskit_node()
            .is_disabled()
    );

    let storage = MemoryStorage::new(b"a,b\n1,2\n");
    let mut fixture = Fixture::new(
        loaded(&storage, configuration()),
        MemoryStorage::new(b"a,b\n1,2\n"),
        Default::default(),
    );
    fixture.run();
    assert!(
        !fixture
            .get_by_label("Detect Format Automatically")
            .accesskit_node()
            .is_disabled()
    );
}

#[test]
fn a_dirty_parsing_change_asks_before_discarding_edits() {
    let storage = MemoryStorage::new(b"a,b\n1,2\n");
    let mut document = loaded(&storage, configuration());
    document.set_cell(0, 0, "edited");
    assert!(document.is_dirty());

    let fixture_storage = MemoryStorage::new(b"a,b\n1,2\n");
    let mut fixture = Fixture::new(document, fixture_storage, Default::default());
    fixture.run();

    fixture
        .get_by_role_and_label(Role::ComboBox, "Separator")
        .click();
    fixture.run();
    fixture.get_by_label("Semicolon (;)").click();
    fixture.run();

    assert!(
        fixture
            .query_by_label("Re-read the file and discard unsaved edits?")
            .is_some(),
        "a dirty re-read must ask first"
    );
    assert_eq!(
        fixture.with_document(|document| document.configuration().column_separator.clone()),
        ",",
        "cancelling the alert must leave the format alone"
    );

    fixture.get_by_label("Cancel").click();
    fixture.run();
    assert!(
        fixture
            .query_by_label("Re-read the file and discard unsaved edits?")
            .is_none()
    );
    assert_eq!(
        fixture.with_document(|document| document.configuration().column_separator.clone()),
        ","
    );
}

#[test]
fn confirming_a_dirty_reparse_replaces_the_table() {
    let storage = MemoryStorage::new(b"a;b\n1;2\n");
    let mut document = loaded(
        &storage,
        CsvConfiguration {
            column_separator: ",".to_owned(),
            ..configuration()
        },
    );
    document.set_cell(0, 0, "edited");
    assert!(document.is_dirty());

    let mut fixture = Fixture::new(
        document,
        MemoryStorage::new(b"a;b\n1;2\n"),
        Default::default(),
    );
    fixture.run();
    fixture
        .get_by_role_and_label(Role::ComboBox, "Separator")
        .click();
    fixture.run();
    fixture.get_by_label("Semicolon (;)").click();
    fixture.run();
    fixture.get_by_label("Re-read").click();
    fixture.settle();

    assert_eq!(
        fixture.with_document(|document| {
            (
                document.configuration().column_separator.clone(),
                document.row_source().expect("source").column_count(),
            )
        }),
        (";".to_owned(), 2)
    );
    assert!(
        fixture
            .query_by_label("Re-read the file and discard unsaved edits?")
            .is_none()
    );
}

#[test]
fn a_failed_reparse_reports_format_not_applied_and_keeps_the_table() {
    let storage = MemoryStorage::new(b"a,b\n1,2\n");
    let mut fixture = Fixture::new(
        loaded(&storage, configuration()),
        MemoryStorage::new(b"a,b\n1,2\n"),
        Default::default(),
    );
    fixture.run();
    let before =
        fixture.with_document(|document| document.row_source().expect("source").row_count());

    fixture.storage().fail_reads();
    fixture
        .get_by_role_and_label(Role::ComboBox, "Separator")
        .click();
    fixture.run();
    fixture.get_by_label("Semicolon (;)").click();
    fixture.settle();

    assert!(
        fixture.query_by_label("Format Not Applied").is_some(),
        "the failure must be visible"
    );
    assert_eq!(
        fixture.with_document(|document| document.row_source().expect("source").row_count()),
        before,
        "the table must survive a failed re-read"
    );
    assert_eq!(
        fixture.with_document(|document| document.configuration().column_separator.clone()),
        ","
    );
}

#[test]
fn choosing_detect_applies_the_detected_format() {
    let storage = MemoryStorage::new(b"a;b\n1;2\n");
    let mut fixture = Fixture::new(
        loaded(&storage, configuration()),
        MemoryStorage::new(b"a;b\n1;2\n"),
        Default::default(),
    );
    fixture.run();

    fixture.get_by_label("Detect Format Automatically").click();
    fixture.settle();

    assert_eq!(
        fixture.with_document(|document| document.configuration().column_separator.clone()),
        ";"
    );
    assert_eq!(
        fixture.with_document(|document| document.row_source().expect("source").column_count()),
        2
    );
}

#[test]
fn the_error_overlay_reports_the_failure_and_detects_a_recovery() {
    let storage = MemoryStorage::new(b"a,b\n1,2\n");
    let mut fixture = Fixture::new(
        read_failed(&storage, configuration()),
        MemoryStorage::new(b"a,b\n1,2\n"),
        Default::default(),
    );
    fixture.run();

    assert!(
        fixture
            .query_by_label("The file could not be read")
            .is_some()
    );
    assert!(
        fixture
            .query_by_label("The file is not valid text in the selected encoding.")
            .is_some()
    );
    assert!(
        fixture
            .query_by_label("Choose another encoding, or detect the format automatically.")
            .is_some()
    );
    assert!(fixture.query_by_label("Add Row Below").is_none());

    fixture.get_by_label("Detect Automatically").click();
    fixture.settle();

    assert_eq!(
        fixture.with_document(|document| *document.phase()),
        tabtool_document::DocumentPhase::Ready
    );
    assert!(
        fixture
            .query_by_label("The file could not be read")
            .is_none()
    );
}

#[test]
fn the_format_bar_can_be_hidden() {
    let mut fixture = Fixture::new_with(
        untitled(configuration()),
        MemoryStorage::new(b""),
        Default::default(),
        tabtool_ui::UiHarness::WIDE,
        false,
        None,
    );
    fixture.run();

    assert!(
        fixture
            .query_by_role_and_label(Role::ComboBox, "Encoding")
            .is_none()
    );
    assert!(
        fixture
            .query_by_label("1 row × 3 columns · Unicode (UTF-8) · Comma")
            .is_some()
    );
}

#[test]
fn a_large_save_offers_cancel_in_the_status_line() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let path = directory.path().join("large.csv");
    let copy = directory.path().join("copy.csv");
    std::fs::write(&path, b"a,b\n1,2\n3,4\n").expect("fixture file is written");
    let mut document = crate::support::large_ready(&path);
    document
        .start_large_save(Some(&copy))
        .expect("start the save");

    let mut fixture = Fixture::new(document, MemoryStorage::new(b""), Default::default());
    fixture.run();

    assert!(
        fixture
            .query_all_by_label_contains("Saving…")
            .next()
            .is_some(),
        "the status line must show the save"
    );
    fixture.get_by_label("Cancel").click();
    fixture.run();

    assert!(
        fixture
            .take_actions()
            .contains(&DocumentAction::Large(LargeUiAction::CancelSave)),
        "Cancel must ask the window to cancel the save"
    );
    fixture.with_document_mut(|document| {
        document.cancel_large_save();
        let _ = document.wait_for_large_save();
    });
}
