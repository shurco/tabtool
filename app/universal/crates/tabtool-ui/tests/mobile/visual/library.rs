//! The Documents library snapshots: empty, populated and a read failure.

use tabtool_i18n::ERROR_OPEN;
use tabtool_ui::Theme;
use tabtool_ui::UiHarness;
use tabtool_ui::harness::SnapshotResults;
use tabtool_ui::mobile::{LibraryError, MobileMetrics};
use tempfile::TempDir;

use super::support::{FOLD_OUTER, PHONE, TABLET, VisualFiles, library_harness};

fn populated(directory: &TempDir) -> VisualFiles {
    let files = VisualFiles::new(directory.path());
    files.write("Budget.csv", "Name,Amount\nAlice,1\n", 1_700_000_300);
    files.write("Résumé.tsv", "Name\tAmount\nBob\t2\n", 1_700_000_200);
    files.write("データ.csv", "名前,値\n花子,3\n", 1_700_000_100);
    files.write("Launch 🎉.txt", "item\n1\n", 1_700_000_000);
    files
}

fn render(mut harness: UiHarness<'static>, name: &str, results: &mut SnapshotResults) {
    harness.run();
    harness.run();
    harness.snapshot(name);
    results.extend(harness.take_snapshot_results());
}

pub fn empty_phone(results: &mut SnapshotResults) {
    let directory = TempDir::new().expect("a temporary documents folder");
    let files = VisualFiles::new(directory.path());
    render(
        library_harness(Theme::desktop_light(), PHONE, MobileMetrics::phone(), files),
        "mobile-library-empty-phone",
        results,
    );
}

pub fn empty_tablet(results: &mut SnapshotResults) {
    let directory = TempDir::new().expect("a temporary documents folder");
    let files = VisualFiles::new(directory.path());
    render(
        library_harness(
            Theme::desktop_light(),
            TABLET,
            MobileMetrics::tablet(),
            files,
        ),
        "mobile-library-empty-tablet",
        results,
    );
}

pub fn list_phone(results: &mut SnapshotResults) {
    let directory = TempDir::new().expect("a temporary documents folder");
    let files = populated(&directory);
    render(
        library_harness(Theme::desktop_light(), PHONE, MobileMetrics::phone(), files),
        "mobile-library-list-phone",
        results,
    );
}

pub fn list_tablet(results: &mut SnapshotResults) {
    let directory = TempDir::new().expect("a temporary documents folder");
    let files = populated(&directory);
    render(
        library_harness(
            Theme::desktop_light(),
            TABLET,
            MobileMetrics::tablet(),
            files,
        ),
        "mobile-library-list-tablet",
        results,
    );
}

pub fn list_folded_outer(results: &mut SnapshotResults) {
    let directory = TempDir::new().expect("a temporary documents folder");
    let files = populated(&directory);
    render(
        library_harness(
            Theme::desktop_light(),
            FOLD_OUTER,
            MobileMetrics::phone(),
            files,
        ),
        "mobile-library-list-fold-outer",
        results,
    );
}

pub fn read_error(results: &mut SnapshotResults) {
    let directory = TempDir::new().expect("a temporary documents folder");
    let files = VisualFiles::new(directory.path());
    files.push(LibraryError::new(ERROR_OPEN, "permission denied"));
    render(
        library_harness(Theme::desktop_light(), PHONE, MobileMetrics::phone(), files),
        "mobile-library-read-error-phone",
        results,
    );
}
