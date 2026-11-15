//! Consolidated visual baselines for the Android/mobile interface.
//!
//! The inventory renders the empty and populated library, a read error, the
//! editor with a column header, row selection and an open native edit, the row
//! and column action bars, the five sheets and large indexing, saving and
//! staleness on the phone (360×800). The states that change most across widths
//! repeat on the tablet (800×1280), the folded outer display (411×797) and the
//! open inner display (841×701 with an active vertical hinge): the library,
//! the editor header, the column action bar and the Format, Settings and
//! Export sheets.
//!
//! The images depend on the operating system's fonts and GPU rasterizer, so
//! comparison is opt-in: set `TABTOOL_RENDER_SNAPSHOTS=1` to render and compare
//! them on this machine. No baseline is committed. Generate a local baseline
//! with `TABTOOL_RENDER_SNAPSHOTS=1 UPDATE_SNAPSHOTS=1`; review every changed
//! image before keeping it. Scenario and accessibility tests live in the other
//! test binaries and run on every platform without a flag.

mod support;

mod editor;
mod library;
mod sheets;

use tabtool_ui::harness::{SnapshotResults, render_snapshots_enabled};

fn finish(results: SnapshotResults) {
    let errors = results.into_inner();
    assert!(errors.is_empty(), "snapshot failures: {errors:?}");
}

#[test]
fn library_states_match_the_baselines() {
    if !render_snapshots_enabled() {
        return;
    }
    let mut results = SnapshotResults::new();
    library::empty_phone(&mut results);
    library::empty_tablet(&mut results);
    library::list_phone(&mut results);
    library::list_tablet(&mut results);
    library::list_folded_outer(&mut results);
    library::read_error(&mut results);
    finish(results);
}

#[test]
fn editor_states_match_the_baselines() {
    if !render_snapshots_enabled() {
        return;
    }
    let mut results = SnapshotResults::new();
    editor::header_phone(&mut results);
    editor::header_tablet(&mut results);
    editor::header_folded_inner(&mut results);
    editor::selection(&mut results);
    editor::editing(&mut results);
    finish(results);
}

#[test]
fn action_bar_states_match_the_baselines() {
    if !render_snapshots_enabled() {
        return;
    }
    let mut results = SnapshotResults::new();
    editor::action_bar_row(&mut results);
    editor::action_bar_column_phone(&mut results);
    editor::action_bar_column_tablet(&mut results);
    editor::action_bar_column_folded(&mut results);
    finish(results);
}

#[test]
fn large_file_states_match_the_baselines() {
    if !render_snapshots_enabled() {
        return;
    }
    let mut results = SnapshotResults::new();
    editor::large_indexing(&mut results);
    editor::large_saving(&mut results);
    editor::large_stale(&mut results);
    finish(results);
}

#[test]
fn sheet_states_match_the_baselines() {
    if !render_snapshots_enabled() {
        return;
    }
    let mut results = SnapshotResults::new();
    sheets::format_phone(&mut results);
    sheets::format_tablet(&mut results);
    sheets::filters_phone(&mut results);
    sheets::settings_phone(&mut results);
    sheets::settings_folded_inner(&mut results);
    sheets::about_phone(&mut results);
    sheets::export_phone(&mut results);
    sheets::export_tablet(&mut results);
    finish(results);
}
