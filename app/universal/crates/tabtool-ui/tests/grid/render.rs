//! Virtualization read counts and render snapshots.

use egui::vec2;
use tabtool_core::{CsvTable, SortDirection};
use tabtool_ui::grid::{CellEditor, GridOptions, GridState, SortSpec};
use tabtool_ui::harness::kittest::Queryable;

use crate::support::{CountingSource, GridFixture, sample_table};

#[test]
fn only_the_visible_cells_of_a_large_source_are_read() {
    let source = CountingSource::new(1_000_000, 20);
    let mut fixture = GridFixture::new(
        &source,
        GridState::default(),
        GridOptions::default(),
        [640.0, 400.0],
    );
    fixture.run();
    fixture.run();
    source.reset();
    fixture.run();

    let calls = source.calls();
    let (min_row, max_row, min_column, max_column) = source.read_bounds().unwrap();
    let read_rows = max_row - min_row + 1;
    let read_columns = max_column - min_column + 1;
    eprintln!(
        "large row count: read {calls} cells, rows {min_row}..={max_row}, columns \
         {min_column}..={max_column}"
    );

    assert_eq!(calls, read_rows * read_columns);
    assert!(calls < 500, "read {calls} cells for a 400 pt viewport");
    assert!(max_row < 30, "read past the visible rows: {max_row}");
    assert!(max_column < 20);
}

#[test]
fn scrolling_far_reads_only_the_rows_around_the_viewport() {
    let source = CountingSource::new(1_000_000, 8);
    let mut state = GridState::default();
    state.scroll = vec2(480.0, 999_000.0 * 24.0);
    let mut fixture = GridFixture::new(&source, state, GridOptions::default(), [640.0, 400.0]);
    fixture.run();
    fixture.run();
    source.reset();
    fixture.run();

    let calls = source.calls();
    let (min_row, max_row, min_column, max_column) = source.read_bounds().unwrap();
    eprintln!(
        "scrolled large row count: read {calls} cells, rows {min_row}..={max_row}, columns \
         {min_column}..={max_column}"
    );

    assert!(calls < 500);
    assert!(min_row >= 998_990, "started at row {min_row}");
    assert!(max_row < 1_000_000, "read row {max_row} past the source");
    assert!(
        min_column >= 2,
        "horizontal scroll read column {min_column}"
    );
    assert!(max_column < 8);
}

#[cfg(target_os = "macos")]
fn snapshot(
    name: &str,
    source: &dyn tabtool_core::RowSource,
    state: GridState,
    options: GridOptions,
) {
    if std::env::var_os("TABTOOL_SKIP_SNAPSHOTS").is_some() {
        return;
    }
    let mut fixture = GridFixture::new(source, state, options, [640.0, 400.0]);
    fixture.run();
    fixture.snapshot(name);
}

#[cfg(target_os = "macos")]
#[test]
fn empty_one_by_one_matches_the_snapshot() {
    let source = CsvTable::new_document(1, 1);
    snapshot(
        "grid-empty-1x1",
        &source,
        GridState::default(),
        GridOptions::default(),
    );
}

#[cfg(target_os = "macos")]
#[test]
fn header_and_data_match_the_snapshot() {
    let source = sample_table();
    let options = GridOptions {
        sort: Some(SortSpec {
            column: 1,
            direction: SortDirection::Ascending,
        }),
        ..GridOptions::default()
    };
    snapshot("grid-header-data", &source, GridState::default(), options);
}

#[cfg(target_os = "macos")]
#[test]
fn the_focused_cell_draws_a_two_pixel_border() {
    if std::env::var_os("TABTOOL_SKIP_SNAPSHOTS").is_some() {
        return;
    }
    let source = sample_table();
    let mut fixture = GridFixture::new(
        &source,
        GridState::default(),
        GridOptions::default(),
        [640.0, 400.0],
    );
    fixture.run();
    fixture.get_by_label("Bob").click();
    fixture.run();
    fixture.run();
    fixture.remove_cursor();
    fixture.run();
    fixture.snapshot("grid-focused-cell");
}

#[cfg(target_os = "macos")]
#[test]
fn selected_row_matches_the_snapshot() {
    let source = sample_table();
    let mut state = GridState::default();
    state.selection.rows.insert(1);
    snapshot("grid-selected-row", &source, state, GridOptions::default());
}

#[cfg(target_os = "macos")]
#[test]
fn selected_column_matches_the_snapshot() {
    let source = sample_table();
    let mut state = GridState::default();
    state.selection.columns.insert(1);
    snapshot(
        "grid-selected-column",
        &source,
        state,
        GridOptions::default(),
    );
}

#[cfg(target_os = "macos")]
#[test]
fn editing_cell_matches_the_snapshot() {
    let source = sample_table();
    let mut state = GridState::default();
    state.editing = Some(CellEditor::new(1, 1, "20"));
    snapshot("grid-editing-cell", &source, state, GridOptions::default());
}

#[cfg(target_os = "macos")]
#[test]
fn hidden_gutter_matches_the_snapshot() {
    let source = sample_table();
    let options = GridOptions {
        shows_row_numbers: false,
        ..GridOptions::default()
    };
    snapshot("grid-hidden-gutter", &source, GridState::default(), options);
}

#[cfg(target_os = "macos")]
#[test]
fn large_mode_header_offset_matches_the_snapshot() {
    let source = sample_table();
    let options = GridOptions {
        header_offset: 1,
        ..GridOptions::default()
    };
    snapshot(
        "grid-large-header-offset",
        &source,
        GridState::default(),
        options,
    );
}

#[test]
fn dragging_the_header_edge_resizes_the_column() {
    let source = sample_table();
    let mut fixture = GridFixture::new(
        &source,
        GridState::default(),
        GridOptions::default(),
        [640.0, 400.0],
    );
    fixture.run();
    let rect = fixture.get_by_label("Name").rect();
    let start = egui::pos2(rect.max.x - 1.0, rect.center().y);
    let end = start + egui::vec2(60.0, 0.0);

    fixture.hover_at(start);
    fixture.run();
    fixture.drag_at(start);
    fixture.run();
    fixture.hover_at(end);
    fixture.run();
    fixture.drop_at(end);
    fixture.run();

    let width = fixture.state.borrow().widths[0];
    assert!(width > 160.0, "the dragged column stayed at {width} pt");
}
