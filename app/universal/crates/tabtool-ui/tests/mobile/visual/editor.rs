//! The touch editor snapshots: header, selection, editing, the action bars,
//! the fold split and the large-file states.

use tabtool_ui::harness::SnapshotResults;
use tabtool_ui::mobile::MobileMetrics;
use tempfile::TempDir;

use crate::support::{self, EditorFixture, small_document};

use super::support::{FOLD_INNER, HINGE, TABLET};

fn render(mut fixture: EditorFixture, name: &str, results: &mut SnapshotResults) {
    fixture.run();
    fixture.run();
    fixture.snapshot(name);
    results.extend(fixture.take_snapshot_results());
}

pub fn header_phone(results: &mut SnapshotResults) {
    let mut fixture = EditorFixture::phone(small_document());
    fixture.run();
    fixture.click_header("Name");
    render(fixture, "mobile-editor-header-phone", results);
}

pub fn header_tablet(results: &mut SnapshotResults) {
    let mut fixture = EditorFixture::new(small_document(), TABLET, MobileMetrics::tablet());
    fixture.run();
    fixture.click_header("Name");
    render(fixture, "mobile-editor-header-tablet", results);
}

pub fn header_folded_inner(results: &mut SnapshotResults) {
    let mut fixture = EditorFixture::new(small_document(), FOLD_INNER, MobileMetrics::tablet());
    fixture.run();
    fixture.set_fold(Some(HINGE));
    fixture.click_header("Name");
    render(fixture, "mobile-editor-header-fold-inner", results);
}

pub fn selection(results: &mut SnapshotResults) {
    let mut fixture = EditorFixture::phone(small_document());
    fixture.run();
    fixture.click_cell("Bob");
    render(fixture, "mobile-editor-selection-phone", results);
}

pub fn editing(results: &mut SnapshotResults) {
    let mut fixture = EditorFixture::phone(small_document());
    fixture.run();
    fixture.click_cell("Alice");
    fixture.click_cell("Alice");
    fixture.set_keyboard(300.0);
    render(fixture, "mobile-editor-editing-phone", results);
}

pub fn action_bar_row(results: &mut SnapshotResults) {
    let mut fixture = EditorFixture::phone(small_document());
    fixture.run();
    fixture.click_cell("Bob");
    render(fixture, "mobile-action-bar-row-phone", results);
}

pub fn action_bar_column_phone(results: &mut SnapshotResults) {
    let mut fixture = EditorFixture::phone(small_document());
    fixture.run();
    fixture.click_header("Amount");
    render(fixture, "mobile-action-bar-column-phone", results);
}

pub fn action_bar_column_tablet(results: &mut SnapshotResults) {
    let mut fixture = EditorFixture::new(small_document(), TABLET, MobileMetrics::tablet());
    fixture.run();
    fixture.click_header("Amount");
    render(fixture, "mobile-action-bar-column-tablet", results);
}

pub fn action_bar_column_folded(results: &mut SnapshotResults) {
    let mut fixture = EditorFixture::new(small_document(), FOLD_INNER, MobileMetrics::tablet());
    fixture.run();
    fixture.set_fold(Some(HINGE));
    fixture.click_header("Amount");
    render(fixture, "mobile-action-bar-column-fold-inner", results);
}

pub fn large_indexing(results: &mut SnapshotResults) {
    let directory = TempDir::new().expect("a temporary directory");
    let mut document = support::large_document(&directory);
    document.force_indexing_state_for_test(0.42);
    document.poll_large_indexing();
    let mut fixture = EditorFixture::phone(document).keep(directory);
    fixture.run();
    render(fixture, "mobile-editor-large-indexing-phone", results);
}

pub fn large_saving(results: &mut SnapshotResults) {
    let mut document = small_document();
    document.force_saving_state_for_test(0.63);
    let mut fixture = EditorFixture::phone(document);
    fixture.run();
    render(fixture, "mobile-editor-large-saving-phone", results);
}

pub fn large_stale(results: &mut SnapshotResults) {
    let directory = TempDir::new().expect("a temporary directory");
    let mut document = support::large_document(&directory);
    let path = directory.path().join("large.csv");
    std::fs::write(&path, "Name,Amount\nAlice,1\nBob,2\nCarol,3\nDora,4\n")
        .expect("the file grows");
    document.refresh_large_staleness();
    let mut fixture = EditorFixture::phone(document).keep(directory);
    fixture.run();
    render(fixture, "mobile-editor-large-stale-phone", results);
}
