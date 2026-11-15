//! The five mobile sheets: Format, Filters, Settings, About and Export.

use tabtool_settings::Settings;
use tabtool_ui::Theme;
use tabtool_ui::UiHarness;
use tabtool_ui::harness::SnapshotResults;
use tabtool_ui::mobile::MobileMetrics;
use tempfile::TempDir;

use crate::support::{file_document, header_configuration, small_document};

use super::support::{
    FOLD_INNER, PHONE, TABLET, VisualFiles, about_harness, export_harness, filters_harness,
    format_harness, settings_harness,
};

fn render(mut harness: UiHarness<'static>, name: &str, results: &mut SnapshotResults) {
    harness.run();
    harness.run();
    harness.snapshot(name);
    results.extend(harness.take_snapshot_results());
}

fn format_document(directory: &TempDir) -> tabtool_document::Document {
    file_document(
        directory.path(),
        "Budget.csv",
        b"Name,Amount\nAlice,1\nBob,2\nCarol,3\n",
        header_configuration(),
    )
}

pub fn format_phone(results: &mut SnapshotResults) {
    let directory = TempDir::new().expect("a temporary directory");
    render(
        format_harness(
            Theme::desktop_light(),
            PHONE,
            MobileMetrics::phone(),
            format_document(&directory),
        ),
        "mobile-sheet-format-phone",
        results,
    );
}

pub fn format_tablet(results: &mut SnapshotResults) {
    let directory = TempDir::new().expect("a temporary directory");
    render(
        format_harness(
            Theme::desktop_light(),
            TABLET,
            MobileMetrics::tablet(),
            format_document(&directory),
        ),
        "mobile-sheet-format-tablet",
        results,
    );
}

pub fn filters_phone(results: &mut SnapshotResults) {
    render(
        filters_harness(
            Theme::desktop_light(),
            PHONE,
            MobileMetrics::phone(),
            small_document(),
        ),
        "mobile-sheet-filters-phone",
        results,
    );
}

pub fn settings_phone(results: &mut SnapshotResults) {
    render(
        settings_harness(
            Theme::desktop_light(),
            PHONE,
            MobileMetrics::phone(),
            Settings::default(),
        ),
        "mobile-sheet-settings-phone",
        results,
    );
}

pub fn settings_folded_inner(results: &mut SnapshotResults) {
    render(
        settings_harness(
            Theme::desktop_light(),
            FOLD_INNER,
            MobileMetrics::tablet(),
            Settings::default(),
        ),
        "mobile-sheet-settings-fold-inner",
        results,
    );
}

pub fn about_phone(results: &mut SnapshotResults) {
    render(
        about_harness(
            Theme::desktop_light(),
            PHONE,
            MobileMetrics::phone(),
            "1.0.0",
        ),
        "mobile-sheet-about-phone",
        results,
    );
}

pub fn export_phone(results: &mut SnapshotResults) {
    let directory = TempDir::new().expect("a temporary documents folder");
    let files = VisualFiles::new(directory.path());
    render(
        export_harness(
            Theme::desktop_light(),
            PHONE,
            MobileMetrics::phone(),
            format_document(&directory),
            files,
        ),
        "mobile-sheet-export-phone",
        results,
    );
}

pub fn export_tablet(results: &mut SnapshotResults) {
    let directory = TempDir::new().expect("a temporary documents folder");
    let files = VisualFiles::new(directory.path());
    render(
        export_harness(
            Theme::desktop_light(),
            TABLET,
            MobileMetrics::tablet(),
            format_document(&directory),
            files,
        ),
        "mobile-sheet-export-tablet",
        results,
    );
}
