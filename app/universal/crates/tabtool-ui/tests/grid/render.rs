//! Virtualization read counts and render snapshots.

use egui::{Rect, pos2, vec2};
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
    let viewport = fixture.state.borrow().viewport;
    assert_eq!(
        viewport,
        Rect::from_min_size(pos2(0.0, 0.0), vec2(624.0, 384.0))
    );
    assert_eq!(source.read_bounds(), Some((0, 15, 0, 6)));
    eprintln!("large row count: read {calls} cells, viewport {viewport:?}");
    assert_eq!(calls, 16 * 7);
    assert_eq!(calls, 112);
}

#[test]
fn scrolling_far_reads_only_the_rows_around_the_viewport() {
    let source = CountingSource::new(1_000_000, 20);
    let mut state = GridState::default();
    state.scroll = vec2(480.0, 999_000.0 * 24.0);
    let mut fixture = GridFixture::new(&source, state, GridOptions::default(), [640.0, 400.0]);
    fixture.run();
    fixture.run();
    source.reset();
    fixture.run();

    let calls = source.calls();
    let viewport = fixture.state.borrow().viewport;
    assert_eq!(viewport.min, pos2(480.0, 999_000.0 * 24.0));
    assert_eq!(viewport.size(), vec2(624.0, 384.0));
    assert_eq!(source.read_bounds(), Some((998_999, 999_015, 3, 11)));
    eprintln!("scrolled large row count: read {calls} cells, viewport {viewport:?}");
    assert_eq!(calls, 17 * 9);
    assert_eq!(calls, 153);
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

    let width = fixture.state.borrow().ordered_widths(&source)[0];
    assert!(width > 100.0, "the dragged column stayed at {width} pt");
}

#[cfg(target_os = "macos")]
#[test]
fn scrolled_chrome_matches_the_snapshot() {
    if std::env::var_os("TABTOOL_SKIP_SNAPSHOTS").is_some() {
        return;
    }
    let headers = (0..12).map(|index| format!("Column {index}")).collect();
    let source = CountingSource::with_headers(100, 12, Some(headers));
    let mut state = GridState::default();
    state.scroll = vec2(100.0, 200.0);
    let mut fixture = GridFixture::new(&source, state, GridOptions::default(), [640.0, 400.0]);
    fixture.run();

    let header = fixture.get_by_label("Column 1").rect();
    let cell = fixture.get_by_label("r9c1").rect();
    let row_number = fixture.get_by_label("10").rect();
    assert!(
        header.min.y < cell.min.y,
        "the header moved with the vertical scroll"
    );
    assert!(
        row_number.min.x < cell.min.x,
        "the row-number gutter moved with the horizontal scroll"
    );

    fixture.remove_cursor();
    fixture.run();
    fixture.snapshot("grid-scrolled-chrome");
}
