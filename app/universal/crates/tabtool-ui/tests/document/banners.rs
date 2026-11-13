//! Large-file, stale, needs-attention, notice and error banners.

use std::io::Write;

use egui::accesskit::Role;
use tabtool_core::{CsvConfiguration, Encoding};
use tabtool_document::{DocumentError, DocumentNotice, LargeStatus};
use tabtool_ui::UiHarness;
use tabtool_ui::document::{DocumentAction, LargeUiAction};
use tabtool_ui::harness::SnapshotResults;
use tabtool_ui::harness::kittest::{NodeT, Queryable};

use crate::support::{Fixture, MemoryStorage, loaded, read_failed};

fn configuration() -> CsvConfiguration {
    CsvConfiguration::default()
}

fn large_fixture() -> (tempfile::TempDir, Fixture) {
    let directory = tempfile::tempdir().expect("temporary directory");
    let path = directory.path().join("large.csv");
    std::fs::write(&path, b"a,b\n1,2\n3,4\n4,5\n").expect("fixture file is written");
    let document = crate::support::large_ready(&path);
    let fixture = Fixture::new(document, MemoryStorage::new(b""), Default::default());
    (directory, fixture)
}

#[test]
fn large_mode_explains_what_stays_available() {
    let (_directory, mut fixture) = large_fixture();
    fixture.run();

    assert!(
        fixture
            .query_by_label_contains("Large file mode. You can browse and edit cells.")
            .is_some(),
        "the large-file banner must be visible"
    );
}

#[test]
fn a_large_ready_document_requests_a_periodic_staleness_poll() {
    let (_directory, mut fixture) = large_fixture();
    fixture.run();

    let delay = fixture.output().viewport_output[&egui::ViewportId::ROOT].repaint_delay;
    assert!(
        delay > std::time::Duration::ZERO && delay <= std::time::Duration::from_secs(1),
        "a large document at rest must poll for staleness, got {delay:?}"
    );
}

#[test]
fn an_external_change_is_seen_without_a_user_action() {
    let (directory, mut fixture) = large_fixture();
    fixture.run();

    std::fs::OpenOptions::new()
        .append(true)
        .open(directory.path().join("large.csv"))
        .expect("open the fixture for append")
        .write_all(b"7,8\n")
        .expect("append a row");

    fixture.run();

    assert!(
        fixture
            .query_by_label("New rows were appended. Reload to see them.")
            .is_some(),
        "the periodic poll must reveal the changed file"
    );
}

#[test]
fn a_stale_large_file_offers_reload() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let path = directory.path().join("large.csv");
    std::fs::write(&path, b"a,b\n1,2\n3,4\n").expect("fixture file is written");
    let mut document = crate::support::large_ready(&path);
    std::fs::OpenOptions::new()
        .append(true)
        .open(&path)
        .expect("open the fixture for append")
        .write_all(b"5,6\n")
        .expect("append a row");
    document.refresh_large_staleness();
    assert_eq!(
        document.large_status(),
        LargeStatus::Stale { appended: true }
    );

    let mut fixture = Fixture::new(document, MemoryStorage::new(b""), Default::default());
    fixture.run();

    assert!(
        fixture
            .query_by_label("New rows were appended. Reload to see them.")
            .is_some()
    );
    assert!(fixture.query_by_label("Reload").is_some());

    fixture.get_by_label("Reload").click();
    fixture.run();
    assert!(
        fixture
            .take_actions()
            .contains(&DocumentAction::Large(LargeUiAction::ReloadStale))
    );

    fixture.with_document_mut(|document| {
        document
            .reload_large(fixture.storage())
            .expect("reload the stale file");
    });
    fixture.run();
    assert!(
        fixture
            .query_by_label("New rows were appended. Reload to see them.")
            .is_none()
    );
}

#[test]
fn a_replaced_large_file_says_it_changed_on_disk() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let path = directory.path().join("large.csv");
    std::fs::write(&path, b"a,b\n1,2\n3,4\n").expect("fixture file is written");
    let mut document = crate::support::large_ready(&path);
    std::fs::write(&path, b"x,y\n").expect("replace the fixture");
    document.refresh_large_staleness();

    let mut fixture = Fixture::new(document, MemoryStorage::new(b""), Default::default());
    fixture.run();

    assert!(
        fixture
            .query_by_label("The file changed on disk. Reload to continue.")
            .is_some()
    );
}

#[test]
fn needs_attention_explains_an_unencodable_cell_and_offers_undo() {
    let storage = MemoryStorage::new(b"a,b\n1,2\n");
    let mut document = loaded(
        &storage,
        CsvConfiguration {
            encoding: Encoding::Windows1252,
            ..configuration()
        },
    );
    document.set_cell(0, 0, "→");
    assert!(document.write_issue().is_some());

    let mut fixture = Fixture::new(document, MemoryStorage::new(b""), Default::default());
    fixture.run();

    assert!(
        fixture
            .query_by_label("This document can’t be saved in its current encoding.")
            .is_some()
    );
    assert!(
        fixture
            .query_by_label_contains("Some text does not exist in the selected encoding.")
            .is_some()
    );
    let undo = fixture.get_by_label("Undo Last Edit");
    assert!(!undo.accesskit_node().is_disabled());

    undo.click();
    fixture.settle();
    assert!(
        fixture
            .query_by_label("This document can’t be saved in its current encoding.")
            .is_none(),
        "undoing the edit must clear the banner"
    );
}

#[test]
fn needs_attention_explains_an_ambiguous_no_quote_document() {
    let storage = MemoryStorage::new(b"a,b\n1,2\n");
    let mut document = loaded(
        &storage,
        CsvConfiguration {
            quote_character: String::new(),
            escape_character: String::new(),
            ..configuration()
        },
    );
    document.set_cell(0, 0, "a,b");
    assert!(document.write_issue().is_some());

    let mut fixture = Fixture::new(document, MemoryStorage::new(b""), Default::default());
    fixture.run();

    assert!(
        fixture
            .query_by_label("This document can’t be saved with quotes off.")
            .is_some()
    );
    assert!(
        fixture
            .query_by_label_contains(
                "A cell contains the separator or a line break and quotes are off."
            )
            .is_some()
    );
}

#[test]
fn show_format_controls_asks_the_window_to_reveal_the_bar() {
    let storage = MemoryStorage::new(b"a,b\n1,2\n");
    let mut document = loaded(
        &storage,
        CsvConfiguration {
            encoding: Encoding::Windows1252,
            ..configuration()
        },
    );
    document.set_cell(0, 0, "→");

    let mut fixture = Fixture::new_with(
        document,
        MemoryStorage::new(b""),
        Default::default(),
        UiHarness::WIDE,
        false,
        None,
    );
    fixture.run();
    assert!(
        fixture
            .query_by_role_and_label(Role::ComboBox, "Encoding")
            .is_none()
    );

    fixture.get_by_label("Show Format Controls").click();
    fixture.run();

    assert!(
        fixture
            .take_actions()
            .contains(&DocumentAction::ShowFormatBar)
    );
}

#[test]
fn a_notice_is_shown_and_can_be_dismissed() {
    let storage = MemoryStorage::new(b"a,b\n1,2\n");
    let ui_state = tabtool_ui::document::DocumentUiState {
        notice: Some(DocumentNotice::CopyLimitExceeded),
        ..Default::default()
    };
    let mut fixture = Fixture::new(
        loaded(&storage, configuration()),
        MemoryStorage::new(b""),
        ui_state,
    );
    fixture.run();

    assert!(fixture.query_by_label("Edit Not Applied").is_some());
    assert!(
        fixture
            .query_by_label("Too many cells selected to copy (limit 1,000,000).")
            .is_some()
    );

    fixture.get_by_label("OK").click();
    fixture.settle();
    assert!(
        fixture.with_ui_state(|state| state.notice.is_none()),
        "OK must dismiss the notice"
    );
    assert!(fixture.query_by_label("Edit Not Applied").is_none());
}

#[test]
fn an_edit_rejected_notice_explains_the_typed_error() {
    let storage = MemoryStorage::new(b"a,b\n1,2\n");
    let ui_state = tabtool_ui::document::DocumentUiState {
        notice: Some(DocumentNotice::EditRejected(
            tabtool_core::CsvError::RowNotEditable,
        )),
        ..Default::default()
    };
    let mut fixture = Fixture::new(
        loaded(&storage, configuration()),
        MemoryStorage::new(b""),
        ui_state,
    );
    fixture.run();

    assert!(fixture.query_by_label("Edit Not Applied").is_some());
    assert!(
        fixture
            .query_by_label_contains("This row cannot be edited right now.")
            .is_some()
    );
}

#[test]
fn a_failed_initial_read_blocks_the_toolbar_and_offers_detect() {
    let storage = MemoryStorage::new(b"a,b\n1,2\n");
    let mut fixture = Fixture::new(
        read_failed(&storage, configuration()),
        MemoryStorage::new(b"a,b\n1,2\n"),
        Default::default(),
    );
    fixture.run();

    assert!(fixture.query_by_label("Add Row Below").is_none());
    assert!(fixture.query_by_label("Export").is_none());
    let detect = fixture.get_by_label("Detect Automatically");
    assert_eq!(detect.accesskit_node().role(), Role::Button);
    assert!(!detect.accesskit_node().is_disabled());
}

#[test]
fn a_read_failure_without_a_path_cannot_detect() {
    let mut document = tabtool_document::Document::new(configuration());
    document
        .load_from_bytes(b"\xFF", None, Some(configuration()), false)
        .expect_err("invalid bytes fail to decode");
    let mut fixture = Fixture::new(document, MemoryStorage::new(b""), Default::default());
    fixture.run();

    assert!(
        fixture
            .get_by_label("Detect Automatically")
            .accesskit_node()
            .is_disabled()
    );
}

#[test]
fn a_save_failure_is_reported_above_the_format_bar() {
    let storage = MemoryStorage::new(b"a,b\n1,2\n");
    let mut fixture = Fixture::new_with(
        loaded(&storage, configuration()),
        MemoryStorage::new(b""),
        Default::default(),
        UiHarness::WIDE,
        true,
        Some(tabtool_document::DocumentError::CannotWriteFile),
    );
    fixture.run();

    assert!(
        fixture
            .query_by_label("The file could not be saved")
            .is_some()
    );
    assert!(
        fixture
            .query_by_label_contains("Check that the folder exists")
            .is_some()
    );
}

#[cfg(target_os = "macos")]
fn snapshot(name: &str, mut fixture: Fixture, results: &mut SnapshotResults) {
    if std::env::var_os("TABTOOL_SKIP_SNAPSHOTS").is_some() {
        return;
    }
    fixture.run();
    fixture.snapshot(name);
    results.extend(fixture.take_snapshot_results());
}

#[cfg(target_os = "macos")]
fn snapshot_pair(name: &str, build: impl Fn([f32; 2]) -> Fixture, results: &mut SnapshotResults) {
    snapshot(
        &format!("document-{name}-1200x800"),
        build(UiHarness::WIDE),
        results,
    );
    snapshot(
        &format!("document-{name}-640x400"),
        build(UiHarness::COMPACT),
        results,
    );
}

#[cfg(target_os = "macos")]
#[test]
fn document_states_match_their_snapshots() {
    use crate::support::untitled;

    let mut results = SnapshotResults::new();
    snapshot_pair(
        "normal",
        |size| {
            Fixture::sized(
                untitled(CsvConfiguration::default()),
                MemoryStorage::new(b""),
                Default::default(),
                size,
            )
        },
        &mut results,
    );

    snapshot_pair(
        "large",
        |size| {
            let directory = tempfile::tempdir().expect("temporary directory");
            let path = directory.path().join("large.csv");
            std::fs::write(&path, b"a,b\n1,2\n3,4\n").expect("fixture file is written");
            let document = crate::support::large_ready(&path);
            Fixture::sized(document, MemoryStorage::new(b""), Default::default(), size)
                .keep(directory)
        },
        &mut results,
    );

    snapshot_pair(
        "stale",
        |size| {
            let directory = tempfile::tempdir().expect("temporary directory");
            let path = directory.path().join("large.csv");
            std::fs::write(&path, b"a,b\n1,2\n3,4\n").expect("fixture file is written");
            let mut document = crate::support::large_ready(&path);
            std::fs::OpenOptions::new()
                .append(true)
                .open(&path)
                .expect("open for append")
                .write_all(b"5,6\n")
                .expect("append");
            document.refresh_large_staleness();
            Fixture::sized(document, MemoryStorage::new(b""), Default::default(), size)
                .keep(directory)
        },
        &mut results,
    );

    snapshot_pair(
        "needs-attention",
        |size| {
            let storage = MemoryStorage::new(b"a,b\n1,2\n");
            let mut document = loaded(
                &storage,
                CsvConfiguration {
                    encoding: Encoding::Windows1252,
                    ..configuration()
                },
            );
            document.set_cell(0, 0, "→");
            Fixture::sized(document, MemoryStorage::new(b""), Default::default(), size)
        },
        &mut results,
    );

    snapshot_pair(
        "error",
        |size| {
            let storage = MemoryStorage::new(b"a,b\n1,2\n");
            Fixture::sized(
                read_failed(&storage, configuration()),
                MemoryStorage::new(b"a,b\n1,2\n"),
                Default::default(),
                size,
            )
        },
        &mut results,
    );

    snapshot_pair(
        "notice",
        |size| {
            let storage = MemoryStorage::new(b"a,b\n1,2\n");
            let ui_state = tabtool_ui::document::DocumentUiState {
                notice: Some(DocumentNotice::CopyLimitExceeded),
                ..Default::default()
            };
            Fixture::sized(
                loaded(&storage, configuration()),
                MemoryStorage::new(b""),
                ui_state,
                size,
            )
        },
        &mut results,
    );

    snapshot_pair(
        "save-error",
        |size| {
            let storage = MemoryStorage::new(b"a,b\n1,2\n");
            Fixture::new_with(
                loaded(&storage, configuration()),
                MemoryStorage::new(b""),
                Default::default(),
                size,
                true,
                Some(DocumentError::CannotWriteFile),
            )
        },
        &mut results,
    );

    snapshot_pair(
        "format-alert",
        |size| {
            let storage = MemoryStorage::new(b"a,b\n1,2\n");
            let ui_state = tabtool_ui::document::DocumentUiState {
                alert: Some(tabtool_ui::document::ChromeAlert {
                    title: "Format Not Applied",
                    message: tabtool_ui::document::alert_message(
                        tabtool_i18n::Language::En,
                        DocumentError::CannotReadFile,
                    ),
                }),
                ..Default::default()
            };
            Fixture::sized(
                loaded(&storage, configuration()),
                MemoryStorage::new(b""),
                ui_state,
                size,
            )
        },
        &mut results,
    );

    snapshot_pair(
        "reparse",
        |size| {
            let storage = MemoryStorage::new(b"a,b\n1,2\n");
            let ui_state = tabtool_ui::document::DocumentUiState {
                pending_reparse: Some(CsvConfiguration {
                    column_separator: ";".to_owned(),
                    ..configuration()
                }),
                ..Default::default()
            };
            Fixture::sized(
                loaded(&storage, configuration()),
                MemoryStorage::new(b""),
                ui_state,
                size,
            )
        },
        &mut results,
    );

    snapshot_pair(
        "saving",
        |size| {
            let directory = tempfile::tempdir().expect("temporary directory");
            let path = directory.path().join("large.csv");
            std::fs::write(&path, b"a,b\n1,2\n3,4\n").expect("fixture file is written");
            let mut document = crate::support::large_ready(&path);
            document.force_saving_state_for_test(0.42);
            Fixture::sized(document, MemoryStorage::new(b""), Default::default(), size)
                .keep(directory)
        },
        &mut results,
    );

    let errors = results.into_inner();
    assert!(errors.is_empty(), "snapshot failures: {errors:?}");
}
