//! Consolidated visual parity baselines for the desktop UI.
//!
//! Every state from Plan 4 is rendered at 1200×800 and 640×400 in both
//! appearances: the new document, the CSV grid, row/column selection, the cell
//! editor, the format and filter bars, the banners and alerts, large-file
//! indexing/saving/staleness, the four Settings tabs, About, Export and the
//! root empty state.
//!
//! Image comparison is macOS-only; other platforms run the scenario and
//! accessibility suites instead. Set `TABTOOL_SKIP_SNAPSHOTS=1` to skip the
//! image comparison on a macOS machine without a usable Metal device.
//! `UPDATE_SNAPSHOTS=1` rewrites the committed images after a reviewed change.

#![cfg(target_os = "macos")]

mod support;

#[path = "visual/documents.rs"]
mod documents;
#[path = "visual/fixture.rs"]
mod fixture;
#[path = "visual/panels.rs"]
mod panels;

use tabtool_ui::Theme;
use tabtool_ui::UiHarness;
use tabtool_ui::harness::SnapshotResults;

use fixture::DocumentFixture;

const SIZES: [(&str, [f32; 2]); 2] = [("1200x800", [1200.0, 800.0]), ("640x400", [640.0, 400.0])];

fn appearances() -> [(&'static str, Theme); 2] {
    [
        ("light", Theme::desktop_light()),
        ("dark", Theme::desktop_dark()),
    ]
}

type DocumentState = fn(Theme, [f32; 2]) -> DocumentFixture;
type HarnessState = fn(Theme, [f32; 2]) -> UiHarness<'static>;

const DOCUMENT_STATES: [(&str, DocumentState); 12] = [
    ("new-document", documents::new_document),
    ("csv-grid-header", documents::csv_with_header),
    ("format-bar", documents::format_bar),
    ("filter-bar", documents::filter_bar),
    ("needs-attention", documents::needs_attention),
    ("read-error", documents::read_error),
    ("save-error", documents::save_error),
    ("format-alert", documents::format_alert),
    ("reparse-confirm", documents::reparse),
    ("large-indexing", documents::large_indexing),
    ("saving", documents::saving),
    ("stale", documents::stale),
];

const SELECTION_STATES: [(&str, DocumentState); 3] = [
    ("selected-row", documents::selected_row),
    ("selected-column", documents::selected_column),
    ("editing-cell", documents::editing_cell),
];

fn snapshots_skipped() -> bool {
    std::env::var_os("TABTOOL_SKIP_SNAPSHOTS").is_some()
}

fn finish(results: SnapshotResults) {
    let errors = results.into_inner();
    assert!(errors.is_empty(), "snapshot failures: {errors:?}");
}

fn snapshot_harness(name: String, mut harness: UiHarness<'static>, results: &mut SnapshotResults) {
    harness.snapshot(name);
    results.extend(harness.take_snapshot_results());
}

#[test]
fn document_states_match_the_baselines() {
    if snapshots_skipped() {
        return;
    }
    let mut results = SnapshotResults::new();
    for (state, build) in DOCUMENT_STATES {
        for (theme_name, theme) in appearances() {
            for (size_name, size) in SIZES {
                let mut fixture = build(theme, size);
                fixture.settle();
                fixture.snapshot(format!("{state}-{size_name}-{theme_name}"), &mut results);
            }
        }
    }
    finish(results);
}

#[test]
fn grid_selection_states_match_the_baselines() {
    if snapshots_skipped() {
        return;
    }
    let mut results = SnapshotResults::new();
    for (state, build) in SELECTION_STATES {
        for (theme_name, theme) in appearances() {
            for (size_name, size) in SIZES {
                let mut fixture = build(theme, size);
                fixture.settle();
                fixture.snapshot(format!("{state}-{size_name}-{theme_name}"), &mut results);
            }
        }
    }
    finish(results);
}

#[test]
fn settings_tabs_match_the_baselines() {
    if snapshots_skipped() {
        return;
    }
    let mut results = SnapshotResults::new();
    for (slug, tab) in [
        ("general", "General"),
        ("new-document", "New Document"),
        ("grid", "Grid"),
        ("large-files", "Large Files"),
    ] {
        for (theme_name, theme) in appearances() {
            for (size_name, size) in SIZES {
                let harness = panels::settings_harness(theme, size, tab);
                snapshot_harness(
                    format!("settings-{slug}-{size_name}-{theme_name}"),
                    harness,
                    &mut results,
                );
            }
        }
    }
    finish(results);
}

#[test]
fn panel_states_match_the_baselines() {
    if snapshots_skipped() {
        return;
    }
    let states: [(&str, HarnessState); 2] = [
        ("about", panels::about_harness),
        ("export", panels::export_harness),
    ];
    let mut results = SnapshotResults::new();
    for (state, build) in states {
        for (theme_name, theme) in appearances() {
            for (size_name, size) in SIZES {
                snapshot_harness(
                    format!("{state}-{size_name}-{theme_name}"),
                    build(theme, size),
                    &mut results,
                );
            }
        }
    }
    finish(results);
}

#[test]
fn the_empty_root_matches_the_baselines() {
    if snapshots_skipped() {
        return;
    }
    let mut results = SnapshotResults::new();
    for (theme_name, theme) in appearances() {
        for (size_name, size) in SIZES {
            snapshot_harness(
                format!("welcome-{size_name}-{theme_name}"),
                panels::welcome_harness(theme, size),
                &mut results,
            );
        }
    }
    finish(results);
}
